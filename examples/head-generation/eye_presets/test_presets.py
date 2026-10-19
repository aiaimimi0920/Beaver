import importlib.util
import unittest
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "eye_preset_packager", Path(__file__).with_name("bundle.py")
)
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)


class PresetTests(unittest.TestCase):
    def test_preserved_baseline_and_independent_parameters(self):
        item = packager.load_preset("almond-balanced-796")
        self.assertEqual(item["baseline"], "795")
        self.assertEqual(item["eye_parameters"]["upper_arch"], 1.0)
        self.assertEqual(item["eye_parameters"]["lower_arch"], 0.68)
        self.assertEqual(len(item["baseline_files_sha256"]), 3)

    def test_exact_controlled_changes(self):
        item = packager.load_preset("almond-balanced-796")
        base = json.loads((packager.SURFACE / "emerald-baseline-795-eyes.json").read_text())
        changes = {k: v for k, v in item["eye_parameters"].items() if base.get(k) != v}
        self.assertEqual(set(base), set(item["eye_parameters"]))
        self.assertEqual(changes, dict(upper_arch=1.0, lower_arch=.68,
            upper_roundness=1.5, upper_horizontal_roundness=.8,
            lower_horizontal_roundness=1.15, lower_shoulder=.5))
        metadata = json.loads((packager.SURFACE / "face-baseline-795.json").read_text())
        for key in ("surface_parameters", "style_profile", "blink", "shading_policy"):
            self.assertEqual(item[key], metadata[key])

    def test_cannot_overwrite_retained_recipe_files(self):
        for path in (packager.SURFACE / "emerald-baseline-795-eyes.json",
                     packager.HERE / "almond-balanced-796.json",
                     packager.HERE / "bundle.py"):
            with self.assertRaises(ValueError):
                packager.validate_output_path(path)

    def test_selectable_presets_have_separate_prompts(self):
        for entry in json.loads((packager.HERE / "catalog.json").read_text())["presets"]:
            name = entry["id"]
            item=packager.load_preset(name)
            prompt=packager.HERE/item["prompt_file"]
            self.assertTrue(prompt.is_file())
            self.assertTrue(item["eye_parameters"])
            self.assertEqual(item["shading_policy"],"smooth_proxy")

    def test_native_evidence_matches_selected_parameters(self):
        evidence = json.loads((packager.HERE / "native-evidence.json").read_text())
        catalog = json.loads((packager.HERE / "catalog.json").read_text())
        names = {int(e["id"][-3:]): e["id"] for e in catalog["presets"]}
        for row in evidence["presets"]:
            params = row["native_parameter_snapshot"]
            if row["version"] == 795:
                selected = json.loads((packager.SURFACE /
                    "emerald-baseline-795-eyes.json").read_text())
            else:
                selected = packager.load_preset(names[row["version"]])["eye_parameters"]
            for key, value in selected.items():
                self.assertEqual(params[key], value)
            ratio = (.044 * params["upper_arch"] + .088 * params["lower_arch"]) / (.22 * params["eye_width"])
            self.assertAlmostEqual(row["analytic_aperture_height_width_ratio"], ratio)
            self.assertEqual(row["captures"], 84)
            self.assertGreater(row["native_orientation"]["minimum_orientation_ratio"], 0)

    def test_explicit_selection_required(self):
        for value in ("", "default", "../facial_surface"):
            with self.assertRaises(ValueError):
                packager.load_preset(value)

    def test_cannot_overwrite_baseline(self):
        with self.assertRaisesRegex(ValueError, "Never overwrite"):
            packager.build_request(
                "almond-balanced-796", "assets/aster/general_face_eyes_795/", Path(".")
            )

    def test_both_preserved_model_directories_are_protected(self):
        for version in (795,796):
            with self.assertRaisesRegex(ValueError,"Never overwrite"):
                packager.build_request("slender-flat-797",
                    f"assets/aster/general_face_eyes_{version}/",Path("."))



if __name__ == "__main__":
    unittest.main()
