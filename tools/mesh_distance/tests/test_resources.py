"""Bounded malformed-input, ownership, lifetime and build-identity negatives."""
import gc
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import numpy as np
from test_support import RECEIPT, mesh
from mesh_distance.backend import MAX_GEOMETRY, MAX_QUERIES, TriangleBVH, load_library


class ResourceTests(unittest.TestCase):
    def setUp(self):
        self.vertices = np.eye(3)
        self.triangles = np.array([[0, 1, 2]])
        self.lib = load_library(RECEIPT)
        self.start_count = self.lib.bmd_handle_count()
        self.start_bytes = self.lib.bmd_live_bytes()

    def tearDown(self):
        gc.collect()
        self.assertEqual(self.lib.bmd_handle_count(), self.start_count)
        self.assertEqual(self.lib.bmd_live_bytes(), self.start_bytes)

    def test_invalid_geometry_and_no_leak(self):
        cases = [(self.vertices, [[False, 1, 2]]),
                 (self.vertices, [[0, np.bool_(True), 2]]),
                 (self.vertices, [[0, 1, True]]),
                 (self.vertices, [[0., 1., 2.]]), (self.vertices, [[-1, 1, 2]]),
                 (self.vertices, [[0, 1, 3]]), (self.vertices, []),
                 (np.full((3, 3), np.nan), self.triangles),
                 (np.full((3, 3), np.inf), self.triangles),
                 (np.ones((3, 3), dtype=complex), self.triangles),
                 (self.vertices, np.array([[0, 1, 2**64 - 1]], dtype=np.uint64)),
                 (np.broadcast_to([0., 0., 0.], (MAX_GEOMETRY + 1, 3)), self.triangles)]
        for vertices, triangles in cases:
            with self.assertRaises(ValueError):
                mesh(vertices, triangles)
        for budget in (0, -1, 2**64, 1.0, True):
            with self.assertRaises(ValueError):
                mesh(self.vertices, self.triangles, memory_limit_bytes=budget)

    def test_queries_and_unrepresentable_distance(self):
        with mesh(self.vertices, self.triangles) as index:
            for points in ([[np.nan, 0, 0]], [[np.inf, 0, 0]], [[0, 0]],
                           np.ones((2, 3), dtype=complex),
                           np.broadcast_to([0., 0., 0.], (MAX_QUERIES + 1, 3))):
                with self.assertRaises(ValueError):
                    index.query(points)
            with self.assertRaises(ValueError):
                index.query([[0., 0., 0.]], exhaustive=2)
            result = index.query(np.empty((0, 3)), True)
            self.assertEqual([a.shape for a in result], [(0,), (0,), (0, 3)])
        maximum = np.finfo(np.float64).max
        # Former unrepresentable-distance test now fails earlier at the explicit
        # capability boundary; the extreme geometry is not silently rescaled.
        with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
            mesh(np.full((3, 3), maximum), self.triangles)
        with mesh(self.vertices, self.triangles) as index:
            with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                index.query([[-maximum, -maximum, -maximum]])

    def test_immutable_inputs_and_close(self):
        vertices = self.vertices.copy()
        triangles = self.triangles.copy()
        index = mesh(vertices, triangles)
        original = index.query([[0., 0., 0.]], True)
        vertices[:] = 99
        triangles[:] = 0
        for a, b in zip(original, index.query([[0., 0., 0.]], True)):
            np.testing.assert_array_equal(a, b)
        handle = index._handle
        index.close()
        index.close()  # Python ownership close is intentionally idempotent.
        self.assertEqual(self.lib.bmd_free(handle), -1)  # C stale token rejected.
        with self.assertRaisesRegex(ValueError, "Closed"):
            index.query([[0., 0., 0.]])
        with self.assertRaises(ValueError):
            index.__enter__()

    def test_noncontiguous_and_unaligned_buffers(self):
        raw = bytearray(73)
        vertices = np.ndarray((3, 3), dtype=np.float64, buffer=raw, offset=1)
        vertices[:] = self.vertices
        with mesh(vertices, self.triangles[:, ::-1]) as index:
            query = np.arange(18, dtype=float).reshape(3, 6)[:, ::2]
            self.assertTrue(np.isfinite(index.query(query)).all())

    def test_build_receipt_and_binary_tampering(self):
        original = Path(RECEIPT)
        with tempfile.TemporaryDirectory(prefix="bmd-negative-") as directory:
            directory = Path(directory)
            data = json.loads(original.read_text())
            binary = directory / data["binary"]
            shutil.copyfile(original.parent / data["binary"], binary)
            receipt = directory / "build.json"
            for key, bad in (("abi", 999), ("source_digest", "bad"), ("flags", []),
                             ("numeric_domain_version", 1), ("numeric_domain_version", 2),
                             ("numeric_domain_version", 3), ("numeric_domain_version", 4), ("link_flags", []),
                             ("binary", "../outside.so")):
                changed = dict(data, **{key: bad})
                receipt.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    TriangleBVH(self.vertices, self.triangles, build_receipt=receipt)
            receipt.write_text(json.dumps(data))
            binary.write_bytes(binary.read_bytes() + b"changed")
            with self.assertRaisesRegex(ValueError, "hash"):
                load_library(receipt)


if __name__ == "__main__":
    unittest.main()
