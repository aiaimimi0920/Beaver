"""Candidate-only exact certificates: no native library/build or model fixtures."""
import copy
from fractions import Fraction
import itertools
import json
import os
from pathlib import Path
import struct
import sys
import unittest
from unittest.mock import patch

import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from mesh_distance.exact_certificate import CertificateUnsupported, ExactThresholdTarget
from mesh_distance.exact_predicate import binary64_ratio
from exact_fraction_oracle import squared_distance

IDENTITY = dict(baseline="AUTHOR", surface_kind="actual_triangles", surface_id="synthetic",
                pose_id="neutral", source_sha256="0" * 64, units="m")


def target(vertices, triangles=None, **kwargs):
    return ExactThresholdTarget(np.array(vertices, dtype=np.float64),
                                [[0, 1, 2]] if triangles is None else triangles,
                                identity=IDENTITY, **kwargs)


def certify(index, points, ids=None, threshold=.001):
    return index.certify(np.array(points, dtype=np.float64),
                         [0] * len(points) if ids is None else ids, threshold,
                         expected_target_digest=index.target_digest)


class ExactCertificateTests(unittest.TestCase):
    def compare(self, triangle, points, threshold=.001):
        expected = [squared_distance(triangle, p) <= Fraction(threshold)**2 for p in points]
        for order in itertools.permutations(range(3)):
            index = target(np.asarray(triangle)[list(order)])
            records = certify(index, points, threshold=threshold)
            self.assertEqual([r["status"] == "WITHIN" for r in records], expected)
            for record, point, inside in zip(records, points, expected):
                self.assertEqual(index.verify_within(record, point, 0, threshold), inside)
                if inside:
                    witness = record["witness"]
                    distance = Fraction(int(witness["squared_numerator_hex"], 16),
                                        int(witness["squared_denominator_hex"], 16))
                    self.assertLessEqual(distance, Fraction(threshold)**2)
                    self.assertGreaterEqual(distance, squared_distance(triangle, point))
        return expected

    def test_exact_boundary_plane_edges_vertices_and_zero(self):
        triangle = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]
        heights = [np.nextafter(.001, 0), .001, np.nextafter(.001, np.inf)]
        for x, y in ((.25, .25), (.5, 0.), (0., 0.)):
            self.assertEqual(self.compare(triangle, [[x, y, h] for h in heights]), [True, True, False])
        self.assertEqual(self.compare(triangle, [[-3/32, -4/32, 0.]], 5/32), [True])
        self.assertEqual(self.compare(triangle, [[.25, .25, 0.], [.25, .25, 2.**-100]], 0.),
                         [True, False])

    def test_degenerates_slivers_large_centroid_and_rotations(self):
        triangles = [np.zeros((3, 3)), np.array([[0., 0., 0.], [1., 2., 3.], [2., 4., 6.]]),
                     np.array([[0., 0., 0.], [1., 0., 0.], [1., 1e-15, 0.]]),
                     np.array([[1e40, 1e40, 0.], [0., 0., 0.], [1., 0., 0.]])]
        for triangle in triangles:
            self.compare(triangle, [[.5, .25, .001], [0., 0., 0.], [1., 0., 0.]])
        rng = np.random.default_rng(2026110206)
        for _ in range(6):
            integer = rng.integers(-2**49, 2**49, size=(2, 3), dtype=np.int64)
            triangle = np.vstack([integer, -integer.sum(0)]).astype(float) * 2.**100
            self.assertEqual(self.compare(triangle, [[0., 0., 0.]], 0.), [True])
            rotation, _ = np.linalg.qr(rng.normal(size=(3, 3)))
            small = rng.normal(size=(3, 3)) @ rotation + [7., -7., 14.]
            self.compare(small, [small.mean(0), small[0], small[0] + [.001, 0., 0.]])

    def test_non_nearest_witness_and_unresolved_is_not_fail(self):
        vertices = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.],
                    [0., 0., .0005], [1., 0., .0005], [0., 1., .0005]]
        index = target(vertices, [[0, 1, 2], [3, 4, 5]])
        records = certify(index, [[.25, .25, .0005]], [0])
        self.assertEqual(records[0]["status"], "WITHIN")  # Triangle 1 is nearer.
        far = certify(index, [[.25, .25, .0014]], [0])[0]
        near = certify(index, [[.25, .25, .0014]], [1])[0]
        self.assertEqual(far["status"], "UNRESOLVED")
        self.assertEqual(near["status"], "WITHIN")

    def test_snapshot_bits_identity_and_tamper_rejection(self):
        vertices = np.eye(3); triangles = np.array([[0, 1, 2]])
        label = dict(IDENTITY)
        index = ExactThresholdTarget(vertices, triangles, identity=label)
        digest = index.target_digest
        vertices[:] = 99.; triangles[:] = 0
        label["baseline"] = "RAW"
        self.assertEqual(index.identity["baseline"], "AUTHOR")
        with self.assertRaises(ValueError):
            index._vertices.setflags(write=True)
        with self.assertRaises(ValueError):
            index._triangles.setflags(write=True)
        self.assertEqual(certify(index, [[0., 1., 0.]], threshold=0.)[0]["status"], "WITHIN")
        query = [1., -0., 0.]
        record = certify(index, [query], threshold=-0.)[0]
        self.assertEqual(record["status"], "WITHIN")
        self.assertEqual(record["query_binary64_le_hex"], struct.pack("<ddd", *query).hex())
        self.assertEqual(record["threshold_binary64_le_hex"], struct.pack("<d", -0.).hex())
        self.assertTrue(index.verify_within(json.loads(json.dumps(record)), query, 0, -0.))
        for key, value in (("target_digest", "bad"), ("triangle_id", 1),
                           ("query_binary64_le_hex", "0" * 48), ("threshold_binary64_le_hex", "0" * 16)):
            changed = dict(record, **{key: value})
            self.assertFalse(index.verify_within(changed, query, 0, -0.))
        changed = copy.deepcopy(record); changed["identity"]["baseline"] = "RAW"
        self.assertFalse(index.verify_within(changed, query, 0, -0.))
        raw = dict(IDENTITY, baseline="RAW")
        self.assertNotEqual(ExactThresholdTarget(np.eye(3), [[0, 1, 2]], identity=raw).target_digest, digest)
        with self.assertRaises(ValueError):
            index.certify(np.eye(3), [0, 0, 0], .001, expected_target_digest="wrong")

    def test_resources_and_invalid_inputs_never_geometric_fail(self):
        triangle = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        index = target(triangle, max_integer_bits=128)
        row = certify(index, [[.25, .25, .001]])[0]
        self.assertEqual((row["status"], row["reason"]), ("UNRESOLVED", "integer_resource_limit"))
        with self.assertRaises(CertificateUnsupported):
            certify(index, np.zeros((4097, 3)))
        for ids in ([True], [0.], [-1], [1], np.array([False])):
            with self.assertRaises(ValueError):
                certify(index, [[.25, .25, 0.]], ids)
        for threshold in (True, np.float32(.001), float("nan"), float("inf"), -.001):
            with self.assertRaises(ValueError):
                certify(index, [[0., 0., 0.]], threshold=threshold)
        for triangles in ([[False, 1, 2]], [[0., 1., 2.]], [[0, 1, 3]]):
            with self.assertRaises(ValueError):
                target(triangle, triangles)
        for vertices in (np.full((3, 3), np.nan), np.eye(3, dtype=np.float32)):
            with self.assertRaises(ValueError):
                ExactThresholdTarget(vertices, [[0, 1, 2]], identity=IDENTITY)
        with self.assertRaises(ValueError):
            ExactThresholdTarget(triangle, [[0, 1, 2]], identity=dict(IDENTITY, surface_kind="bilinear"))

    def test_bit_decoder_matches_exact_binary64_ratios(self):
        values = [0., -0., .001, np.nextafter(0., 1.), np.finfo(float).max, -16.]
        rng = np.random.default_rng(19425)
        values += list(rng.normal(size=100))
        for value in values:
            self.assertEqual(binary64_ratio(value), float(value).as_integer_ratio())

    def test_wide_rows_rejected_before_numpy_allocation(self):
        # Tiny spies demonstrate ordering without allocating a hostile large row.
        with patch("mesh_distance.exact_certificate.np.asarray", side_effect=AssertionError("early allocation")):
            for vertices, triangles in (([[0.] * 4], [[0, 0, 0]]),
                                        ([[0., 0., 0.]], [[0, 0, 0, 0]])):
                # The second case gets through vertex conversion first, so test
                # its original-index validator directly without that conversion.
                if len(triangles[0]) == 4:
                    from mesh_distance.exact_certificate import _indices
                    with self.assertRaises(ValueError):
                        _indices(triangles, 3, 1, 4096)
                else:
                    with self.assertRaises(ValueError):
                        ExactThresholdTarget(vertices, triangles, identity=IDENTITY)

    def test_negative_subnormal_gate_and_endian_bits(self):
        index = target(np.eye(3))
        with self.assertRaises(ValueError):
            certify(index, [[1., 0., 0.]], threshold=-np.nextafter(0., 1.))
        vertices = np.eye(3).astype(">f8")
        other = ExactThresholdTarget(vertices, np.array([[0, 1, 2]], dtype=">i8"), identity=IDENTITY)
        self.assertEqual(index.target_digest, other.target_digest)
        query = np.array([[1., -0., np.nextafter(0., 1.)]], dtype=">f8")
        record = other.certify(query, [0], .001, expected_target_digest=other.target_digest)[0]
        self.assertEqual(record["query_binary64_le_hex"], struct.pack("<ddd", 1., -0., np.nextafter(0., 1.)).hex())

    def test_explicit_bvh_uses_the_same_immutable_snapshot(self):
        receipt = os.environ.get("MESH_DISTANCE_BUILD")
        if not receipt:
            self.skipTest("Explicit distance build receipt not supplied")
        vertices = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        index = ExactThresholdTarget(vertices, [[0, 1, 2]], identity=IDENTITY)
        vertices[:] = 99.
        with index.build_bvh(build_receipt=receipt) as distance_target:
            _, ids, _ = distance_target.query([[.25, .25, .001]], True)
        self.assertEqual(certify(index, [[.25, .25, .001]], ids)[0]["status"], "WITHIN")

    def test_untrusted_record_and_metadata_are_bounded_before_serialization(self):
        index = target(np.eye(3))
        query = [1., 0., 0.]
        record = certify(index, [query], threshold=0.)[0]
        extra = dict(record, extra=object())
        long_string = dict(record, target_digest="x" * 1024)
        boolean_id = dict(record, triangle_id=False)
        huge_integer = dict(record, max_integer_bits=1 << 20000)
        with patch("mesh_distance.exact_certificate.json.dumps", side_effect=AssertionError("external serialization")):
            for invalid in (extra, long_string, boolean_id, huge_integer):
                self.assertFalse(index.verify_within(invalid, query, 0, 0.))
            self.assertTrue(index.verify_within(record, query, 0, 0.))
            with self.assertRaises(ValueError):
                ExactThresholdTarget(np.eye(3), [[0, 1, 2]], identity=dict(IDENTITY, extra=object()))


if __name__ == "__main__":
    unittest.main()
