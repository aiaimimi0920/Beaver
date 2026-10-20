import copy
import hashlib
import unittest

from brow_evidence import validate_brow_report, REQUIRED_SOURCES


class BrowEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.design = {"kind": "symbols", "detail_count": 3, "clearance_m": .00065, "thickness_m": .0002}
        self.sources = {name: "source:" + name for name in REQUIRED_SOURCES}
        hashes = {k: hashlib.sha256(v.encode()).hexdigest() for k, v in self.sources.items()}
        self.snapshot = {"brow_design": self.design, "brow_source_sha256": hashes}
        side = {"kind": "symbols", "islands": 3, "manifold_edges": True, "vertices": 18, "triangles": 24, "local_areas_m2": [.001]*3, "signed_volume_m3": 1e-7, "requested_vertex_depth_clearance_m": .00045, "sampled_min_depth_clearance_m": .0003, "clearance_samples": 15}
        self.report = {"brow_design": {"parameters": self.design, "source_sha256": hashes, "sides": [{**side, "side": -1}, {**side, "side": 1}]}}
        self.style = {"brow_design": self.design}

    def verify(self, report=None, style=None, sources=None):
        return validate_brow_report(self.snapshot, report if report is not None else self.report, style if style is not None else self.style, sources if sources is not None else self.sources)

    def test_bound_metrics_and_legacy(self):
        self.assertEqual(self.verify()["side_count"], 2)
        self.assertIsNone(validate_brow_report({}, {}, {}, {}))

    def test_reject_wrong_design_missing_report_or_style(self):
        for kind in ("curl", "arc"):
            r = copy.deepcopy(self.report)
            r["brow_design"]["parameters"]["kind"] = kind
            with self.assertRaisesRegex(ValueError, "mismatch"):
                self.verify(report=r)
        with self.assertRaises(ValueError):
            self.verify(report={})
        with self.assertRaises(ValueError):
            self.verify(style={})
        with self.assertRaises(ValueError):
            validate_brow_report({}, self.report, self.style, self.sources)

    def test_reject_geometry_corruption(self):
        for field, value in [("side", 1), ("vertices", 17), ("triangles", 23), ("islands", 2), ("signed_volume_m3", -1), ("sampled_min_depth_clearance_m", -.00001), ("requested_vertex_depth_clearance_m", .001), ("clearance_samples", 0), ("manifold_edges", False)]:
            r = copy.deepcopy(self.report)
            r["brow_design"]["sides"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify(report=r)
        r = copy.deepcopy(self.report)
        r["brow_design"]["sides"] = []
        with self.assertRaises(ValueError):
            self.verify(report=r)

    def test_reject_frozen_source_change(self):
        with self.assertRaises(ValueError):
            self.verify(sources={"brow_shapes.py": "changed"})


if __name__ == "__main__":
    unittest.main()
