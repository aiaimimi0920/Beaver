"""Explicit, inactive candidate-only exact threshold witnesses for triangle targets."""
import hashlib
import json
from pathlib import Path
import struct

import numpy as np

from .exact_predicate import IntegerBudgetExceeded, binary64_ratio, candidate_within

SCHEMA = "BEAVER_EXACT_TRIANGLE_WITNESS_V1"
MAX_GEOMETRY = 2_000_000
MAX_QUERIES = 4096
MAX_TARGET_BYTES = 128 * 1024 * 1024


class CertificateUnsupported(RuntimeError):
    """Resource policy prevented a certificate; never a geometric FAIL."""
    status = "UNRESOLVED"


def _length(values, maximum, label):
    if not isinstance(values, (np.ndarray, list, tuple)):
        raise ValueError(f"{label} requires an ndarray or list/tuple")
    if isinstance(values, np.ndarray) and values.ndim == 0:
        raise ValueError(f"{label} cannot be scalar")
    if len(values) > maximum:
        raise CertificateUnsupported(f"{label} count limit")
    return len(values)


def _rows(values, columns, scalar_check, label):
    # Validate original rows BEFORE np.asarray or flattening can allocate an
    # unexpectedly wide/ragged structure or erase bool/scalar identity.
    for row in values:
        if (not isinstance(row, (list, tuple, np.ndarray))
                or (isinstance(row, np.ndarray) and row.ndim != 1) or len(row) != columns):
            raise ValueError(f"{label} requires rows of {columns}")
        if any(not scalar_check(value) for value in row):
            raise ValueError(f"{label} scalar type mismatch")


def _points(values, maximum, label):
    _length(values, maximum, label)
    if isinstance(values, np.ndarray):
        if values.ndim != 2 or values.shape[1] != 3:
            raise ValueError(f"{label} must have shape (N, 3)")
    else:
        _rows(values, 3, lambda x: type(x) in (float, np.float64), label)
    array = np.asarray(values)
    if array.ndim != 2 or array.dtype.kind != "f" or array.dtype.itemsize != 8:
        raise ValueError(f"{label} must be binary64 without coercion")
    # Canonicalize endian/order through INTEGER views, not floating conversion.
    bits = array.view(np.dtype(array.dtype.str.replace("f8", "u8")))
    data = bits.astype("<u8", copy=False).tobytes()
    raw = np.frombuffer(data, dtype="<u8")
    if np.any((raw & 0x7ff0000000000000) == 0x7ff0000000000000):
        raise ValueError(f"{label} must be finite binary64")
    return data, np.frombuffer(data, dtype="<f8").reshape(-1, 3)


def _integer(value):
    return not isinstance(value, (bool, np.bool_)) and isinstance(value, (int, np.integer))


def _indices(values, columns, count, maximum):
    _length(values, maximum, "Indices")
    if isinstance(values, np.ndarray):
        if values.ndim != (2 if columns else 1) or (columns and values.shape[1] != columns):
            raise ValueError("Integer index shape mismatch")
    elif columns:
        _rows(values, columns, _integer, "Indices")
    elif any(not _integer(value) for value in values):
        raise ValueError("Integer candidate IDs required before NumPy promotion")
    array = np.asarray(values)
    if (array.dtype.kind not in "iu" or np.any(array < 0) or np.any(array >= count)):
        raise ValueError("In-range integer indices required")
    data = array.astype("<i8", copy=False).tobytes()
    shape = (-1, columns) if columns else (-1,)
    return data, np.frombuffer(data, dtype="<i8").reshape(shape)


def _identity(identity):
    required = {"baseline", "surface_kind", "surface_id", "pose_id", "source_sha256", "units"}
    if (type(identity) is not dict or len(identity) != len(required)
            or any(type(key) is not str for key in identity) or any(key not in identity for key in required)
            or any(type(v) is not str or not 0 < len(v) <= 256 for v in identity.values())
            or identity["baseline"] not in ("RAW", "AUTHOR") or identity["units"] != "m"
            or identity["surface_kind"] not in ("actual_triangles", "bilinear_tessellation_triangles")
            or len(identity["source_sha256"]) != 64
            or any(c not in "0123456789abcdef" for c in identity["source_sha256"])):
        raise ValueError("Explicit RAW/AUTHOR, triangle representation, surface/pose/source identity required")
    return json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()


def _source_digest():
    directory = Path(__file__).resolve().parent
    sources = {name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
               for name in ("exact_certificate.py", "exact_predicate.py")}
    return hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest()


def _same_record(expected, supplied):
    """Bound work by the trusted expected schema; never serialize external data."""
    if type(expected) is dict:
        if (type(supplied) is not dict or len(supplied) != len(expected)
                or any(type(key) is not str for key in supplied)
                or any(key not in supplied for key in expected)):
            return False
        return all(_same_record(value, supplied[key]) for key, value in expected.items())
    if type(expected) in (list, tuple):
        if type(supplied) not in (list, tuple) or len(supplied) != len(expected):
            return False
        return all(_same_record(a, b) for a, b in zip(expected, supplied))
    if type(supplied) is not type(expected):  # bool must not compare equal to int.
        return False
    if type(expected) is str:
        return len(supplied) == len(expected) and supplied == expected
    if type(expected) is int:
        return supplied.bit_length() == expected.bit_length() and supplied == expected
    return expected is None and supplied is None


class ExactThresholdTarget:
    """Own immutable byte snapshots, separate from any rounded-distance API.

    Inputs MUST already be binary64. Identities label the caller's reference
    axis/representation, not field ownership. Their truth must be established by
    the surrounding audit; this class binds them, not authenticates provenance.
    No coordinate normalization, numerical cap, native load or implicit fallback.
    """
    def __init__(self, vertices, triangles, *, identity, max_integer_bits=16384):
        if type(max_integer_bits) is not int or not 128 <= max_integer_bits <= 16384:
            raise ValueError("Integer bit budget must be an integer in [128, 16384]")
        nv = _length(vertices, MAX_GEOMETRY, "Vertices")
        nt = _length(triangles, MAX_GEOMETRY, "Triangles")
        if not nv or not nt:
            raise ValueError("Nonempty target required")
        if (nv + nt) * 24 > MAX_TARGET_BYTES:
            raise CertificateUnsupported("Target byte budget exceeded")
        self._identity_bytes = _identity(identity)
        vb, self._vertices = _points(vertices, MAX_GEOMETRY, "Vertices")
        tb, self._triangles = _indices(triangles, 3, len(self._vertices), MAX_GEOMETRY)
        digest = hashlib.sha256(SCHEMA.encode() + self._identity_bytes)
        digest.update(struct.pack("<QQ", len(self._vertices), len(self._triangles)))
        digest.update(vb)
        digest.update(tb)
        self._target_digest = digest.hexdigest()
        self._source = _source_digest()
        self._max_bits = max_integer_bits

    @property
    def target_digest(self):
        return self._target_digest

    @property
    def identity(self):
        return json.loads(self._identity_bytes)

    def build_bvh(self, *, build_receipt):
        """Explicit opt-in: build the distance index from this SAME byte snapshot.

        Caller owns/ closes the returned BVH. Its separate numeric domain can
        reject targets that exact integer certificates can represent. No fallback.
        """
        from .backend import TriangleBVH
        return TriangleBVH(self._vertices, self._triangles, build_receipt=build_receipt)

    def certify(self, points, candidate_ids, threshold, *, expected_target_digest):
        """Return WITHIN/UNRESOLVED only, bound to exact original input bits.

        A non-nearest candidate can prove WITHIN; no candidate can prove FAIL.
        Integer-budget exhaustion returns UNRESOLVED. Whole-batch/target resource
        limits raise CertificateUnsupported (also UNRESOLVED), not geometry FAIL.
        """
        if expected_target_digest != self._target_digest:
            raise ValueError("Expected target snapshot identity mismatch")
        if type(threshold) not in (float, np.float64):
            raise ValueError("Binary64 threshold required")
        threshold_ratio = binary64_ratio(threshold)
        if threshold_ratio[0] < 0:
            raise ValueError("Nonnegative threshold required")
        _, queries = _points(points, MAX_QUERIES, "Queries")
        _, ids = _indices(candidate_ids, 0, len(self._triangles), MAX_QUERIES)
        if len(queries) != len(ids):
            raise ValueError("One candidate triangle ID per query required")
        threshold_bits = struct.pack("<d", threshold).hex()
        ratio = tuple(hex(x) for x in threshold_ratio)
        records = []
        for point, triangle_id in zip(queries, ids):
            triangle = self._vertices[self._triangles[triangle_id]]
            proof, reason = None, "no_within_witness_on_candidate"
            try:
                proof = candidate_within(triangle, point, threshold, self._max_bits)
            except IntegerBudgetExceeded:
                reason = "integer_resource_limit"
            records.append({"schema": SCHEMA, "status": "WITHIN" if proof else "UNRESOLVED",
                            "reason": "exact_triangle_witness" if proof else reason,
                            "target_digest": self._target_digest, "identity": self.identity,
                            "implementation_digest": self._source, "max_integer_bits": self._max_bits,
                            "query_binary64_le_hex": point.tobytes().hex(),
                            "threshold_binary64_le_hex": threshold_bits,
                            "threshold_ratio_hex": ratio, "triangle_id": int(triangle_id),
                            "witness": proof})
        return records

    def verify_within(self, record, point, candidate_id, threshold):
        """Recompute, not trust, a record; caller supplies its original query/ID/gate."""
        expected = self.certify([point], [candidate_id], threshold,
                                expected_target_digest=self._target_digest)[0]
        return expected["status"] == "WITHIN" and _same_record(expected, record)
