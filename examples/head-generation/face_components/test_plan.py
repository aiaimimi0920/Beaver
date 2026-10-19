import copy
import json
import unittest

from compose import HERE, resolve
from plan import build_plan


class PlanTests(unittest.TestCase):
    def setUp(self):
        self.selection = json.loads((HERE / "example-selection.json").read_text())

    def test_no_evidence_does_not_claim_progress(self):
        plan = build_plan(self.selection, records=[])
        self.assertEqual(plan["current_output_status"], "not_generated")
        self.assertTrue(all(s["status"] == "pending" for s in plan["steps"]))
        self.assertEqual(plan["recorded_evidence"], [])
        self.assertEqual(len(plan["available_option_counts"]), 6)

    def test_exact_snapshot_remains_recorded_not_current_acceptance(self):
        snap = resolve(self.selection)
        record = {**snap, "candidate": "test-only"}
        plan = build_plan(self.selection, records=[record])
        self.assertEqual(plan["recorded_evidence"][0]["match"], "exact_snapshot")
        self.assertEqual(plan["current_output_status"], "not_generated")

    def test_same_ids_with_changed_snapshot_are_historical(self):
        record = {"selection": self.selection, "composition_sha256": "old"}
        result = build_plan(self.selection, records=[record])
        self.assertEqual(
            result["recorded_evidence"][0]["match"], "historical_selection_only"
        )

    def test_other_selection_is_not_evidence(self):
        other = {**self.selection, "brows": "soft-level"}
        self.assertEqual(
            build_plan(self.selection, records=[resolve(other)])["recorded_evidence"],
            [],
        )

    def test_policy_change_does_not_reuse_exact_evidence(self):
        result = build_plan(
            self.selection, "skin-seated-075", [resolve(self.selection)]
        )
        self.assertEqual(
            result["recorded_evidence"][0]["match"], "historical_selection_only"
        )

    def test_catalog_evidence_and_input_preserved(self):
        before = copy.deepcopy(self.selection)
        result = build_plan(self.selection)
        self.assertEqual(before, self.selection)
        self.assertTrue(
            any(x["candidate"] == "combo827" for x in result["recorded_evidence"])
        )
        self.assertFalse(
            any(x["match"] == "exact_snapshot" for x in result["recorded_evidence"])
        )

    def test_current_831_snapshot_identified(self):
        choice = {
            "eyes": "almond",
            "brows": "soft-level",
            "mouth": "wide-neutral",
            "nose": "defined-bridge",
            "ears": "compact-human",
            "face": "wide-face",
        }
        result = build_plan(choice, "skin-seated-075")
        self.assertEqual(
            [
                x["candidate"]
                for x in result["recorded_evidence"]
                if x["match"] == "exact_snapshot"
            ],
            ["combo831"],
        )

    def test_invalid_selection_rejected(self):
        with self.assertRaises(ValueError):
            build_plan({"eyes": "almond"})

    def test_known_failure_warning_is_scoped_and_deterministic(self):
        choice = {
            "eyes": "almond",
            "brows": "soft-level",
            "mouth": "wide-neutral",
            "nose": "defined-bridge",
            "ears": "compact-human",
            "face": "wide-face",
        }
        plan = build_plan(choice)
        self.assertEqual(plan, build_plan(choice))
        self.assertEqual(plan["compatibility_warnings"][0]["candidate"], "combo829")
        self.assertEqual(plan["snapshot"]["fit_policy"], "legacy")
        self.assertIn("Review known failure", plan["next_action"])
        self.assertEqual(
            plan["recorded_evidence"][0]["recorded_fit_policy"], "skin-seated-075"
        )
        self.assertEqual(
            build_plan(choice, "skin-seated-075")["compatibility_warnings"], []
        )
        self.assertFalse(
            any(x["candidate"] == "combo829" for x in plan["recorded_evidence"])
        )
