"""Native packaged calibration, provenance and aggregate budget agree."""
import ast
import json
from pathlib import Path
import tempfile
import unittest
from bundle import build_request


def payloads(source, depth=0):
    result = []
    if depth > 4:
        return result
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return result
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            try:
                data = json.loads(node.value)
                if isinstance(data, dict):
                    result.append(data)
            except ValueError:
                pass
            if "import " in node.value:
                result.extend(payloads(node.value, depth + 1))
    return result


class LipBundleTests(unittest.TestCase):
    def test_packaged_calibration_identity_and_combined_budget(self):
        here = Path(__file__).parent
        preset = json.loads((here.parent / "eye_presets/face-chin-narrow-820.json").read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names = ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + n for n in (
                    "aster_head_editable.blend", "aster_head.glb", "aster_definition.tres",
                    "hair_base.png", "hair_ilm.png")]
            for name in names:
                p = root/name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("CPU fixture")
            def build(lips=None, bound=.0042):
                return build_request(preset["surface_parameters"], preset["eye_parameters"],
                    "assets/aster/general_face_eyes_lip_cpu/", root, bound, "emerald_illustrated", blink=True,
                    silhouette_parameters=preset["silhouette_parameters"], lip_parameters=lips)
            baseline = build()
            self.assertEqual(baseline, build({"upper_lip_relief_scale":1}))
            data = payloads(build({"upper_lip_relief_scale":.9})["code"])
            styles = [p for p in data if "calibration" in p]
            self.assertTrue(styles)
            for style in styles:
                self.assertAlmostEqual(style["calibration"]["upper_lip_relief"], .0279)
            evidence = next(p for p in data if "packager_helpers" in p)
            report = evidence["report"]
            self.assertEqual(evidence["packager_helpers"]["source_lips.py"],
                             (here/"source_lips.py").read_text())
            total = report["surface"]["conservative_support_displacement_bound_m"]
            before = report["surface"]["support_bound_before_lip_relief_m"]
            self.assertAlmostEqual(total-before, report["lip_relief"]["analytic_depth_change_bound_m"])
            with self.assertRaisesRegex(ValueError, "exceeds explicitly reviewed"):
                build({"upper_lip_relief_scale":.9}, total - 1e-8)

            data = payloads(build({"upper_lip_lateral_power":2})["code"])
            for style in [p for p in data if "calibration" in p]:
                self.assertEqual(style["calibration"]["upper_lip_lateral_power"],2)
                self.assertAlmostEqual(style["calibration"]["upper_lip_relief"],.031)
            with self.assertRaisesRegex(ValueError, "exceeds explicitly reviewed"):
                build({"upper_lip_relief_scale":.85,"upper_lip_lateral_power":2})
