"""Explicit optional CPU fenv tests; the helper is never compiled by this test."""
import ctypes
import os
from pathlib import Path
import struct
import sys
import unittest

import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from mesh_distance.exact_certificate import ExactThresholdTarget

IDENTITY = dict(baseline="AUTHOR", surface_kind="actual_triangles", surface_id="synthetic-fenv",
                pose_id="neutral", source_sha256="0" * 64, units="m")


def number(bits):
    return struct.unpack("<d", struct.pack("<Q", bits))[0]


class ExactEnvironmentTests(unittest.TestCase):
    def test_nonfinite_and_negative_subnormal_bit_patterns(self):
        index = ExactThresholdTarget(np.zeros((3, 3)), [[0, 1, 2]], identity=IDENTITY)
        for bits in (0x8000000000000001, 0x7ff0000000000000, 0xfff0000000000000,
                     0x7ff8000000000001, 0xfff8000000000123):
            with self.assertRaises(ValueError):
                index.certify(np.zeros((1, 3)), [0], number(bits), expected_target_digest=index.target_digest)
        for bits in (0x7ff0000000000000, 0x7ff8000000000001):
            with self.assertRaises(ValueError):
                index.certify(np.array([[number(bits), 0., 0.]]), [0], .001,
                              expected_target_digest=index.target_digest)

    def test_daz_ftz_and_rounding_do_not_change_integer_certificates(self):
        path = os.environ.get("EXACT_CERT_FENV_HELPER")
        if not path:
            self.skipTest("Explicit trusted test-only MXCSR helper not supplied")
        helper = ctypes.CDLL(path)
        helper.certificate_test_get_mxcsr.argtypes = []
        helper.certificate_test_get_mxcsr.restype = ctypes.c_uint32
        helper.certificate_test_set_mxcsr.argtypes = [ctypes.c_uint32]
        helper.certificate_test_set_mxcsr.restype = None
        negative_tiny, tiny = number(0x8000000000000001), number(1)
        queries = np.array([[0., 0., 0.], [tiny, 0., 0.], [number(2), 0., 0.]])
        index = ExactThresholdTarget(np.zeros((3, 3)), [[0, 1, 2]], identity=IDENTITY)
        saved = helper.certificate_test_get_mxcsr()
        try:
            for flags in (0, 1 << 6, 1 << 15, (1 << 6) | (1 << 15), 0x2000, 0x4000, 0x6000):
                helper.certificate_test_set_mxcsr((saved & ~0xe040) | flags)
                with self.assertRaises(ValueError):
                    index.certify(queries[:1], [0], negative_tiny, expected_target_digest=index.target_digest)
                records = index.certify(queries, [0, 0, 0], tiny, expected_target_digest=index.target_digest)
                self.assertEqual([r["status"] for r in records], ["WITHIN", "WITHIN", "UNRESOLVED"])
                for point, record in zip(queries, records):
                    self.assertEqual(record["query_binary64_le_hex"], point.tobytes().hex())
        finally:
            helper.certificate_test_set_mxcsr(saved)


if __name__ == "__main__":
    unittest.main()
