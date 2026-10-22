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

    def test_source_packaging_precedes_sdf(self):
        result = build_plan(self.selection, records=[])
        self.assertEqual(
            [step["stage"] for step in result["steps"]],
            ["generate", "source_packaging", "sdf", "capture", "review"],
        )
        self.assertIn(".source-plan.json", result["steps"][0]["required"])
        self.assertEqual(
            result["steps"][1]["entry"], "../source_layout/pack_request.py"
        )
        self.assertIn("--source-packaging-receipt", result["steps"][2]["required"])
        self.assertTrue(all(step["status"] == "pending" for step in result["steps"]))

    def test_researched_brow_samples_are_not_fresh_acceptance(self):
        evidence = json.loads(
            (HERE / "../brow_library/native-evidence.json").read_text()
        )["samples"][0]
        result = build_plan(evidence["selection"], evidence["fit_policy"])
        record = next(
            item
            for item in result["recorded_evidence"]
            if item["candidate"] == evidence["candidate"]
        )
        self.assertEqual(record["recorded_sample_status"], evidence["status"])
        self.assertEqual(record["recorded_scope"], evidence["scope"])
        self.assertEqual(result["current_output_status"], "not_generated")
        self.assertTrue(all(step["status"] == "pending" for step in result["steps"]))

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

    def test_831_snapshot_preserved_as_historical_after_catalog_extension(self):
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
            [],
        )
        self.assertEqual(result["recorded_evidence"][0]["candidate"], "combo831")

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

    def test_brow_samples_remain_visible_after_catalog_updates(self):
        for brow, candidate in (
            ("angular-peaked", "combo832"),
            ("broad-band", "combo833"),
        ):
            result = build_plan({**self.selection, "brows": brow})
            records = result["recorded_evidence"]
            self.assertEqual([r["candidate"] for r in records], [candidate])
            for record in records:
                expected = (
                    "exact_snapshot"
                    if record["composition_sha256"]
                    == result["snapshot"]["composition_sha256"]
                    else "historical_selection_only"
                )
                self.assertEqual(record["match"], expected)
            self.assertEqual(result["current_output_status"], "not_generated")

    def test_template_record_stays_historical_evidence(self):
        selection = {**self.selection, "face": "root-protected-round-chin"}
        result = build_plan(selection)
        record = next(
            r for r in result["recorded_evidence"] if r["candidate"] == "combo845"
        )
        self.assertEqual(record["recorded_generation_status"], "succeeded")
        self.assertEqual(result["current_output_status"], "not_generated")
        self.assertEqual(result["family_inventory"]["face"]["implemented_families"], 3)

    def test_mouth_template_record_is_not_fresh_acceptance(self):
        choice = {**self.selection, "face": "retained", "mouth": "bowed-contour"}
        plan = build_plan(choice)
        row = next(r for r in plan["recorded_evidence"] if r["candidate"] == "combo847")
        self.assertEqual(row["recorded_generation_status"], "succeeded")
        self.assertEqual(plan["current_output_status"], "not_generated")

    def test_nose_template_record_does_not_accept_new_output(self):
        choice = {**self.selection, "face": "retained", "nose": "short-rounded"}
        plan = build_plan(choice)
        row = next(r for r in plan["recorded_evidence"] if r["candidate"] == "combo849")
        self.assertEqual(row["recorded_generation_status"], "succeeded")
        self.assertEqual(plan["current_output_status"], "not_generated")
