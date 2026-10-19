"""Reference-refined candidates remain explicit and preserve retained baselines."""

import json
from pathlib import Path
import unittest
import sys
from unittest.mock import patch
from test_presets import packager

previous_path = sys.path[:]
sys.path.insert(0, str(packager.HERE.parent / "semantic_eyes"))
try:
    from parameters import validate
finally:
    sys.path[:] = previous_path

CANDIDATES = {
    "almond-taper-experiment-803": ("aperture_corner_taper", 1),
    "rounded-arch-shallow-805": ("aperture_upper_share", 0.72),
    "asymmetric-angular-807": ("upper_angularity", 0.65),
    "soft-wing-downturned-808": ("wing_lift_scale", 0.5),
    "clean-downturned-810": ("layered_lashes", 0),
    "clean-angular-811": ("upper_angularity", 0.65),
    "clean-slender-812": ("layered_lashes", 0),
}


class RefinedSelectionTests(unittest.TestCase):
    def test_catalog_and_specs_match(self):
        entries = json.loads((packager.HERE / "catalog.json").read_text())["presets"]
        self.assertEqual(len(entries), len({e["id"] for e in entries}))
        for entry in entries:
            spec = packager.load_preset(entry["id"])
            self.assertEqual(entry["id"], spec["id"])
            self.assertEqual(entry["file"], spec["id"] + ".json")
            self.assertEqual(entry["prompt"], spec["prompt_file"])
            self.assertTrue((packager.HERE / entry["prompt"]).is_file())
        self.assertTrue(set(CANDIDATES) <= {e["id"] for e in entries})

    def test_refined_parameters_are_explicit(self):
        for name, (parameter, expected) in CANDIDATES.items():
            spec = packager.load_preset(name)
            self.assertEqual(spec["eye_parameters"][parameter], expected)
            self.assertEqual(spec["baseline"], "795")

    def test_recorded_native_evidence_covers_refined_selection(self):
        data = json.loads((packager.HERE / "refined-native-evidence.json").read_text())
        rows = {row["id"]: row for row in data["candidates"]}
        self.assertEqual(len(rows), len(data["candidates"]))
        self.assertEqual(set(rows), set(CANDIDATES))
        for name, row in rows.items():
            spec = packager.load_preset(name)
            self.assertEqual(
                validate(row["native_parameter_snapshot"]),
                validate(spec["eye_parameters"]),
            )
            self.assertEqual(
                row["source_sha256"], spec["evidence"]["native_source_before"]
            )
            self.assertEqual(row["captures_verified"], 84)
            self.assertRegex(row["manifest_sha256"], r"^[0-9a-f]{64}$")

    def test_complete_recipe_forwarding(self):
        workspace = Path("/tmp/preset-selection-test")
        for name in CANDIDATES:
            spec = packager.load_preset(name)
            prefix = "assets/aster/general_face_eyes_replay_" + name[-3:] + "/"
            with patch.object(packager.importlib.util, "module_from_spec") as factory:
                module = factory.return_value
                with patch.object(packager.importlib.util, "spec_from_file_location"):
                    packager.build_request(name, prefix, workspace)
                module.build_request.assert_called_once_with(
                    spec["surface_parameters"],
                    spec["eye_parameters"],
                    prefix,
                    workspace,
                    spec["reviewed_support_bound_m"],
                    spec["style_profile"],
                    blink=spec["blink"],
                )

    def test_retained_targets_stay_protected(self):
        for name in CANDIDATES:
            for version in (795, 796):
                with self.assertRaisesRegex(ValueError, "Never overwrite"):
                    packager.build_request(
                        name, f"assets/aster/general_face_eyes_{version}/", Path(".")
                    )

    def test_current_family_choices_resolve_without_overwriting_baseline(self):
        catalog = json.loads((packager.HERE / "catalog.json").read_text())
        choices = catalog["current_family_choices"]
        self.assertEqual(len(choices), 8)
        self.assertEqual(len(set(choices.values())), 8)
        self.assertEqual(choices["round-open"], "baseline-795")
        for family, name in choices.items():
            if family != "round-open":
                self.assertEqual(packager.load_preset(name)["id"], name)

    def test_visually_rejected_candidate_not_selectable(self):
        with self.assertRaises(ValueError):
            packager.load_preset("arched-shallow-experiment-804")


if __name__ == "__main__":
    unittest.main()
