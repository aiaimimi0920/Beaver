"""Opt-in, geometry-only finite-query triangle distance. No generator imports this."""
import ctypes as C
import json
from pathlib import Path
import threading

import numpy as np

from .build_backend import ABI, FLAGS, LINK_FLAGS, sha256, source_identity

MAX_NATIVE_BYTES = 512 * 1024 * 1024
MAX_GEOMETRY = 2_000_000
MAX_QUERIES = 1_000_000


def load_library(build_receipt):
    """Load only an explicitly supplied, hash-bound local build; never compile."""
    path = Path(build_receipt).resolve()
    receipt = json.loads(path.read_text(encoding="utf-8"))
    sources, digest = source_identity()
    if (receipt.get("schema") != "BEAVER_MESH_DISTANCE_BUILD_V1"
            or receipt.get("abi") != ABI or receipt.get("sources") != sources
            or receipt.get("source_digest") != digest
            or receipt.get("flags") != list(FLAGS)
            or receipt.get("link_flags") != list(LINK_FLAGS)
            or receipt.get("numeric_domain_version") != 5
            or receipt.get("binary") != "libbeaver_mesh_distance.so"):
        raise ValueError("Build receipt/source/ABI/flags mismatch")
    binary = path.parent / receipt["binary"]
    if sha256(binary) != receipt["binary_sha256"]:
        raise ValueError("Compiled binary hash mismatch")
    lib = C.CDLL(str(binary))
    for name in ("bmd_abi_version", "bmd_source_sha256", "bmd_error",
                 "bmd_live_bytes", "bmd_handle_count"):
        getattr(lib, name).argtypes = []
    lib.bmd_abi_version.restype = C.c_uint32
    lib.bmd_source_sha256.restype = C.c_char_p
    lib.bmd_error.restype = C.c_char_p
    lib.bmd_live_bytes.restype = C.c_uint64
    lib.bmd_handle_count.restype = C.c_uint64
    if lib.bmd_abi_version() != ABI or lib.bmd_source_sha256().decode() != digest:
        raise ValueError("Loaded native ABI/source identity mismatch")
    f64 = np.ctypeslib.ndpointer(dtype=np.float64, flags=("C_CONTIGUOUS", "ALIGNED"))
    i64 = np.ctypeslib.ndpointer(dtype=np.int64, flags=("C_CONTIGUOUS", "ALIGNED"))
    u64 = np.ctypeslib.ndpointer(dtype=np.uint64, flags=("C_CONTIGUOUS", "ALIGNED"))
    lib.bmd_build.argtypes = [f64, C.c_int64, i64, C.c_int64, C.c_uint64]
    lib.bmd_build.restype = C.c_uint64
    lib.bmd_free.argtypes = [C.c_uint64]
    lib.bmd_free.restype = C.c_int
    lib.bmd_query.argtypes = [C.c_uint64, f64, C.c_int64, f64, i64, f64, u64, C.c_int]
    lib.bmd_query.restype = C.c_int
    return lib


def real_points(values, maximum, label, *, nonempty=False):
    values = np.asarray(values)
    if (values.ndim != 2 or values.shape[1] != 3 or values.dtype.kind not in "fiu"
            or len(values) > maximum or (nonempty and not len(values))):
        raise ValueError(f"{label}: bounded real numeric (N, 3) array required")
    return np.require(values, dtype=np.float64, requirements=("C", "A"))


def validate_index_scalars(values):
    """Do not erase bool/float identities by NumPy list promotion."""
    if isinstance(values, np.ndarray):
        if values.dtype.kind not in "iu":
            raise ValueError("Triangle scalar indices must be integers, not bool")
        return
    if not isinstance(values, (list, tuple)) or len(values) > MAX_GEOMETRY:
        raise ValueError("Triangles require an integer ndarray or bounded list/tuple")
    for row in values:
        if (not isinstance(row, (list, tuple, np.ndarray))
                or (isinstance(row, np.ndarray) and row.ndim != 1) or len(row) != 3):
            raise ValueError("Triangles require rows of three integer indices")
        for index in row:
            if isinstance(index, (bool, np.bool_)) or not isinstance(index, (int, np.integer)):
                raise ValueError("Triangle scalar indices must be integers, not bool/float")


class TriangleBVH:
    """Own an immutable target; close explicitly or use a context manager.

    Coordinates/hits/distances are binary64, not exact real arithmetic. Binary128
    fallback handles ill-conditioned triangles. IDs resolve computed-distance
    ties to the lowest input triangle index; they NEVER define field/UV owners.
    Native calls serialize, create no threads, and reject stale handles. Python
    callers must not concurrently mutate arrays supplied to a query/build.
    """

    def __init__(self, vertices, triangles, *, build_receipt,
                 memory_limit_bytes=MAX_NATIVE_BYTES):
        self._handle = 0
        self._lock = threading.RLock()
        if (type(memory_limit_bytes) is not int
                or not 0 <= memory_limit_bytes <= MAX_NATIVE_BYTES):
            raise ValueError("Native memory budget must be an integer in [0, 512 MiB]")
        v = real_points(vertices, MAX_GEOMETRY, "Vertices", nonempty=True)
        validate_index_scalars(triangles)
        t = np.asarray(triangles)
        if (t.ndim != 2 or t.shape[1] != 3 or not 0 < len(t) <= MAX_GEOMETRY
                or t.dtype.kind not in "iu" or np.any(t < 0) or np.any(t >= len(v))):
            raise ValueError("Triangles must be bounded, nonempty in-range integer (M, 3)")
        t = np.require(t, dtype=np.int64, requirements=("C", "A"))
        self._lib = load_library(build_receipt)
        self._handle = self._lib.bmd_build(v, len(v), t, len(t), memory_limit_bytes)
        if not self._handle:
            raise ValueError(self._lib.bmd_error().decode())

    def close(self):
        with self._lock:
            if self._handle:
                status = self._lib.bmd_free(self._handle)
                if status:
                    raise ValueError(self._lib.bmd_error().decode())
                self._handle = 0

    def __enter__(self):
        if not self._handle:
            raise ValueError("Closed BVH")
        return self

    def __exit__(self, *args):
        self.close()

    def __del__(self):
        try:
            self.close()
        except Exception:
            # Constructor failure/interpreter teardown must not mask an error.
            pass

    def query(self, points, details=False, *, exhaustive=False, statistics=False):
        """Return distances, optionally (distance, triangle_id, hit) and counts.

        Empty queries are valid. `exhaustive` disables spatial pruning for small
        verification cases; it is not a higher-precision or continuous oracle.
        Nonfinite input/unrepresentable output raises, never yields passing NaN.
        No threshold or acceptance epsilon is applied by this backend.
        """
        if type(exhaustive) is not bool:
            raise ValueError("exhaustive must be bool")
        with self._lock:
            if not self._handle:
                raise ValueError("Closed BVH")
            p = real_points(points, MAX_QUERIES, "Queries")
            d = np.empty(len(p), dtype=np.float64)
            ids = np.empty(len(p), dtype=np.int64)
            hits = np.empty_like(p)
            tested = np.empty(len(p), dtype=np.uint64)
            status = self._lib.bmd_query(self._handle, p, len(p), d, ids, hits,
                                         tested, int(exhaustive))
            if status:
                raise ValueError(self._lib.bmd_error().decode())
            result = (d, ids, hits) if details else d
            return (result, tested) if statistics else result
