"""Report provenance and state assertions are independent from GLB binding tests."""

import copy
import importlib.util
import json
import unittest
from mouth_evidence import LIBRARY, NAMES, SOURCES, validate_report
from compose import resolve
from test_mouth_selection import BASE


class MouthEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = resolve(BASE)
        spec = importlib.util.spec_from_file_location(
            "mouth_evidence_fixture_design", LIBRARY / "mouth_design.py"
        )
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
        old = json.loads(
            (LIBRARY.parent / "checkpoints/current/face_style.json").read_text()
        )
        self.style = m.apply(old, self.snapshot["mouth_design"])
        self.frozen = {name: (LIBRARY / name).read_text() for name in SOURCES}
        record = {
            "parameters": self.snapshot["mouth_design"],
            "source_sha256": self.snapshot["mouth_source_sha256"],
            "authored_support_envelope_m": 0.0,
        }
        self.report = {
            "mouth_design": record,
            "facial_surface": {"mouth_design": copy.deepcopy(record)},
            "mouth_motion": {
                "target_names": NAMES,
                "new_targets": NAMES[3:],
                "sampled_combinations": 81,
                "blink_overlap_vertices": 0,
                "outside_owned_components_unchanged": True,
                "sampled_triangles": 1,
                "minimum_sampled_area_m2": 1e-8,
                "minimum_sampled_normal_dot": 1e-12,
                "changed_vertices": {n: 1 for n in NAMES[3:]},
                "max_displacement_m": {n: 0.001 for n in NAMES[3:]},
            },
        }

    def verify(self):
        return validate_report(self.snapshot, self.report, self.style, self.frozen)

    def test_bound_report(self):
        self.assertTrue(self.verify())

    def test_source_tamper(self):
        self.frozen["mouth_motion.py"] += "\n#changed"
        with self.assertRaises(ValueError):
            self.verify()

    def test_calibration_tamper(self):
        self.style["calibration"]["mouth_half_width"] += 0.001
        with self.assertRaises(ValueError):
            self.verify()

    def test_reject_invalid_metrics(self):
        for value in [True, 0, -1, float("nan")]:
            self.report["mouth_motion"]["changed_vertices"]["MouthRound"] = value
            with self.assertRaises(ValueError):
                self.verify()

    def test_reject_unbound_report(self):
        self.snapshot.pop("mouth_design")
        with self.assertRaises(ValueError):
            self.verify()

    def test_reject_overlap_or_missing_samples(self):
        self.report["mouth_motion"]["blink_overlap_vertices"] = 1
        with self.assertRaises(ValueError):
            self.verify()
