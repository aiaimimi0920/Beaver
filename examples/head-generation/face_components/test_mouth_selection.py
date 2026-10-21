"""Researched mouth selection identity and bounded control integration."""

import unittest
from compose import resolve, verify_snapshot
from plan import build_plan

BASE = {
    "eyes": "downturned",
    "brows": "soft-arched",
    "mouth": "research-line",
    "nose": "research-balanced",
    "ears": "retained",
    "face": "narrow-chin",
}


class MouthSelectionTests(unittest.TestCase):
    def test_four_presets_are_bound(self):
        for name in [
            "research-line",
            "research-lip-band",
            "research-round-muzzle",
            "research-short-muzzle",
        ]:
            result = resolve({**BASE, "mouth": name})
            self.assertIsNotNone(result["mouth_design"])
            self.assertEqual(len(result["mouth_source_sha256"]), 5)
            self.assertEqual(verify_snapshot(result), result)

    def test_owned_override_changes_identity(self):
        a = resolve(BASE)
        b = resolve(BASE, mouth_overrides={"half_width": 0.07})
        self.assertNotEqual(a["composition_sha256"], b["composition_sha256"])
        self.assertEqual(b["mouth_design"]["half_width"], 0.07)
        self.assertEqual(b["nose_design"], a["nose_design"])

    def test_legacy_has_no_new_targets(self):
        result = resolve({**BASE, "mouth": "retained"})
        self.assertIsNone(result.get("mouth_design"))
        with self.assertRaises(ValueError):
            resolve({**BASE, "mouth": "retained"}, mouth_overrides={})

    def test_invalid_override(self):
        for item in [
            {"kind": "muzzle"},
            {"version": 2},
            {"half_width": True},
            {"half_width": float("nan")},
            {"unknown": 1},
        ]:
            with self.assertRaises(ValueError):
                resolve(BASE, mouth_overrides=item)

    def test_plan_preserves_requested_controls(self):
        result = build_plan(BASE, records=[], mouth_overrides={"half_width": 0.07})
        self.assertEqual(result["snapshot"]["mouth_design"]["half_width"], 0.07)
        self.assertEqual(result["current_output_status"], "not_generated")
