"""Catalog metadata contracts only; native rendering is tested separately."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from catalog import build_catalog


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.directory = self.root / "assets/aster/general_face_eyes_combo906"
        self.directory.mkdir(parents=True)
        self.selection = dict(eyes="downturned", brows="soft-arched",
                              nose="research-balanced", mouth="research-line",
                              ears="research-standard-oval", face="research-compact-rounded")
        self.record = {"id": "ear-standard-oval", "label": "标准椭圆耳｜紧凑圆脸",
                       "candidate": 906, "selection": self.selection}
        (self.directory / "aster_definition.tres").write_text("CPU fixture, not a native resource")
        (self.directory / "aster_head.glb").write_bytes(b"CPU fixture, not a native model")
        self.report = self.directory / "generation_report.json"
        self.model_hash = hashlib.sha256((self.directory / "aster_head.glb").read_bytes()).hexdigest()
        self.report.write_text(json.dumps({"face_components": {"selection": self.selection},
                                           "new_glb_sha256": self.model_hash}))

    def test_source_bound_catalog_does_not_claim_independent_composition(self):
        result = build_catalog(self.root, [self.record])
        entry = result["entries"][0]
        self.assertEqual(result["schema_version"], 1)
        self.assertEqual(entry["selection"], self.selection)
        self.assertEqual(entry["definition"],
                         "res://assets/aster/general_face_eyes_combo906/aster_definition.tres")
        self.assertEqual(entry["report_sha256"], hashlib.sha256(self.report.read_bytes()).hexdigest())
        self.assertEqual(set(entry) - {"id", "label", "selection", "definition"},
                         {"report_sha256", "model_sha256", "definition_sha256"})

    def test_unknown_or_falsely_labeled_combination_rejected(self):
        record = copy.deepcopy(self.record)
        record["selection"]["face"] = "research-soft-tapered"
        with self.assertRaises(ValueError):
            build_catalog(self.root, [record])
        record = copy.deepcopy(self.record)
        record["candidate"] = 999
        with self.assertRaises(ValueError):
            build_catalog(self.root, [record])

    def test_malformed_bound_report_is_rejected(self):
        for value in (None, "bad", [], {"face_components": None},
                      {"face_components": "bad"}, {"face_components": []}):
            self.report.write_text(json.dumps(value))
            with self.assertRaises(ValueError):
                build_catalog(self.root, [self.record])

    def test_report_cannot_be_rebound_to_another_same_selection_model(self):
        for value in (None, "0" * 64, "", self.model_hash.upper()):
            self.report.write_text(json.dumps({"face_components": {"selection": self.selection},
                                               "new_glb_sha256": value}))
            with self.assertRaises(ValueError):
                build_catalog(self.root, [self.record])

    def test_duplicates_and_invalid_identity_are_rejected(self):
        with self.assertRaises(ValueError):
            build_catalog(self.root, [self.record, self.record])
        for value in (True, "906", -1, 906.5):
            record = {**self.record, "candidate": value}
            with self.assertRaises(ValueError):
                build_catalog(self.root, [record])


if __name__ == "__main__":
    unittest.main()
