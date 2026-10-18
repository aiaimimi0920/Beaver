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

    def test_explicit_selection_required(self):
        for value in ("", "default", "../facial_surface"):
            with self.assertRaises(ValueError):
                packager.load_preset(value)

    def test_cannot_overwrite_baseline(self):
        with self.assertRaisesRegex(ValueError, "Never overwrite"):
            packager.build_request(
                "almond-balanced-796", "assets/aster/general_face_eyes_795/", Path(".")
            )


if __name__ == "__main__":
    unittest.main()
