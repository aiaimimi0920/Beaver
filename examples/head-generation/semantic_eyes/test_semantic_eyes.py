"""CPU recipe/geometry-function tests. No Blender or Godot execution."""

import ast
import copy
import json
import math
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from parameters import adapt_style, validate, LIMITS, upper_change
from bundle import build_request, RECIPE, REQUIRED

TARGET = {
    "eye_spacing": 1.0,
    "eye_width": 1.04,
    "upper_arch": 0.78,
    "lower_arch": 0.70,
    "corner_slope_offset": 0.002,
    "eye_height_offset": 0.0,
    "iris_width": 0.96,
    "iris_height": 0.92,
    "brow_thickness": 0.70,
    "brow_arch": 1.10,
    "brow_height_offset": 0.004,
    "brow_rise_offset": 0.004,
    "lash_band_thickness": 0.65,
    "supplemental_lashes": 0,
    "lid_fold": 0,
}


class SemanticEyeTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())

    def contour(self, style):
        tree = ast.parse((RECIPE / "face.py").read_text())
        node = next(
            n
            for n in tree.body
            if isinstance(n, ast.FunctionDef) and n.name == "eye_contour"
        )
        c = style["calibration"]
        scope = {"C": c, "H": c["height_m"], "Y0": c["origin_y_m"]}
        exec(
            compile(
                ast.Module(body=[node], type_ignores=[]), "<actual eye contour>", "exec"
            ),
            scope,
        )
        return scope["eye_contour"]

    def test_identity_copy_and_invalid(self):
        original = copy.deepcopy(self.style)
        self.assertEqual(adapt_style(self.style, {}), original)
        altered = adapt_style(self.style, TARGET)
        self.assertEqual(self.style, original)
        self.assertNotEqual(altered, original)
        for key, value in [
            ("unknown", 1),
            ("eye_width", True),
            ("upper_arch", math.nan),
            ("eye_width", 1.2),
            ("supplemental_lashes", 0.5),
        ]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                adapt_style(self.style, {key: value})

    def test_target_actual_contour_ratios_and_coupling(self):
        changed = adapt_style(self.style, TARGET)
        base, target = self.contour(self.style), self.contour(changed)
        for side in (-1, 1):
            self.assertAlmostEqual(target(side, 0, True)[0], base(side, 0, True)[0])
            self.assertAlmostEqual(
                (target(side, 1, True)[0] - target(side, -1, True)[0])
                / (base(side, 1, True)[0] - base(side, -1, True)[0]),
                1.04,
            )
        old, new = self.style["calibration"], changed["calibration"]
        for key, scale in [("eye_upper_arch", 0.78), ("eye_lower_arch", 0.70)]:
            self.assertAlmostEqual(new[key] / old[key], scale)
        self.assertAlmostEqual(
            changed["eye_half_width_m"], new["eye_half_width"] * new["height_m"]
        )
        self.assertAlmostEqual(
            changed["iris_half_width_m"], new["iris_half_width"] * new["height_m"]
        )
        self.assertEqual(old["mouth_height"], new["mouth_height"])
        self.assertEqual(old["nose_stations"], new["nose_stations"])
        for old_row, new_row in zip(
            self.style["lid_fold_height_profile"], changed["lid_fold_height_profile"]
        ):
            t = -0.76 + 1.52 * old_row[0]
            expected = old_row[1] + upper_change(t, old, validate(TARGET)) + 0.002 * t
            self.assertAlmostEqual(new_row[1], expected)
        for key in LIMITS:
            for bound in LIMITS[key][1:]:
                # Single-axis supported endpoints must be valid; combinations
                # are additionally protected by the geometric envelope guard.
                request = {key: bound}
                if key in ("layered_lashes", "upper_lash_layer") and bound:
                    request.update(
                        short_upper_tuft=1,
                        supplemental_lashes=0,
                        lash_contour_follow=1,
                        layered_lashes=1,
                    )
                if key == "secondary_lash_scale" and bound != 1:
                    request.update(
                        short_upper_tuft=1, supplemental_lashes=0, lash_contour_follow=1
                    )
                if key == "short_upper_tuft" and bound:
                    request.update(supplemental_lashes=0, lash_contour_follow=1)
                if key in ("upper_asymmetry", "lower_shoulder") and bound:
                    request["corner_slope_offset"] = -self.style["calibration"][
                        "eye_corner_slope"
                    ]
                adapt_style(self.style, request)

    def test_nasal_spacing_extension_preserves_width_and_local_features(self):
        old = adapt_style(self.style, {"eye_spacing": 0.96})
        new = adapt_style(self.style, {"eye_spacing": 0.94})
        a, b = old["calibration"], new["calibration"]
        shift = b["eye_center_x"] - a["eye_center_x"]
        self.assertLess(shift, 0)
        self.assertEqual(a["eye_half_width"], b["eye_half_width"])
        self.assertGreater(b["eye_center_x"] - b["eye_half_width"], 0)
        for key in ("iris_center_x", "brow_inner_x", "brow_outer_x"):
            self.assertAlmostEqual(b[key] - a[key], shift)
        for key in ("nose_stations", "mouth_height", "eye_corner_y"):
            self.assertEqual(a[key], b[key])
        with self.assertRaises(ValueError):
            validate({"eye_spacing": 0.939})

    def test_retained_layout_configs(self):
        root = Path(__file__).resolve().parent.parent / "facial_surface"
        a = json.loads((root / "emerald-narrow-layout-eyes.json").read_text())
        b = json.loads((root / "emerald-balanced-layout-eyes.json").read_text())
        self.assertEqual(validate(a)["eye_spacing"], 0.94)
        self.assertEqual(validate(b)["eye_spacing"], 0.94)
        self.assertEqual([key for key in a if a[key] != b[key]], ["lower_arch"])
        self.assertEqual(b["lower_arch"], 1.0)

    def test_roundness_tracks_actual_upper_rim(self):
        request = dict(
            TARGET,
            upper_roundness=1.6,
            upper_horizontal_roundness=0.72,
            upper_arch=1.05,
        )
        changed = adapt_style(self.style, request)
        base, target = self.contour(self.style), self.contour(changed)
        H = self.style["calibration"]["height_m"]
        for i in range(101):
            t = -1 + i / 50
            actual = (target(1, t, True)[1] - base(1, t, True)[1]) / H
            expected = (
                upper_change(t, self.style["calibration"], validate(request))
                + request["corner_slope_offset"] * t
            )
            self.assertAlmostEqual(actual, expected, places=12)
        self.assertEqual(
            changed["calibration"]["nose_stations"],
            self.style["calibration"]["nose_stations"],
        )

    def test_coherent_lash_rail_is_tapered_and_bounded(self):
        request = dict(
            TARGET,
            upper_roundness=1.6,
            upper_horizontal_roundness=0.72,
            upper_arch=1.05,
            lash_contour_follow=1,
        )
        changed = adapt_style(self.style, request)
        rails = changed["upper_lash_rails"]
        self.assertEqual(len(rails), 21)
        self.assertAlmostEqual(rails[0][1], rails[0][2])
        self.assertAlmostEqual(rails[-1][1], rails[-1][2])
        for row in rails[1:-1]:
            self.assertGreater(row[2], row[1])
            self.assertLess(row[2] - row[1], 0.02)
            self.assertTrue(all(math.isfinite(x) for x in row))
        self.assertTrue(all(a[0] < b[0] for a, b in zip(rails, rails[1:])))
        self.assertEqual(
            changed["calibration"]["ear_outward_span"],
            self.style["calibration"]["ear_outward_span"],
        )

    def test_simple_iris_controls_keep_brows_and_other_features(self):
        regular = adapt_style(self.style, TARGET)
        simple = adapt_style(
            self.style, dict(TARGET, simple_iris=1, iris_vertical_offset=0.006)
        )
        self.assertEqual(simple["iris_graphic_disc_strength"], 0)
        self.assertEqual(simple["iris_accent_strength"], 0)
        self.assertAlmostEqual(
            simple["calibration"]["iris_center_y"]
            - regular["calibration"]["iris_center_y"],
            0.006,
        )
        self.assertEqual(
            simple["calibration"]["brow_height"], regular["calibration"]["brow_height"]
        )
        self.assertAlmostEqual(
            simple["calibration"]["pupil_half_height"]
            / regular["calibration"]["pupil_half_height"],
            0.84,
        )
        self.assertAlmostEqual(
            simple["calibration"]["catchlight_half_width"]
            / regular["calibration"]["catchlight_half_width"],
            0.8,
        )

    def test_optional_geometry_is_removed_before_assembly(self):
        tree = ast.parse(Path(__file__).with_name("runtime.py").read_text())
        tree.body = [
            n for n in tree.body if not isinstance(n, (ast.Import, ast.ImportFrom))
        ]
        removed, repaired = [], []
        scope = {
            "bpy": SimpleNamespace(
                data=SimpleNamespace(
                    objects=SimpleNamespace(
                        remove=lambda obj, do_unlink: removed.append(obj.name)
                    )
                )
            ),
            "store_mouth_deltas": lambda obj: repaired.append(obj.name),
        }
        exec(compile(tree, "<native selection mock>", "exec"), scope)
        names = [
            "Upper lash 1",
            "Upper skin lid 1",
            "Upper swept lash tuft 1",
            "Embedded lateral child 0 1",
            "Tapered upper eyelid fold 1",
            "Pupil separate accent 1",
        ]
        parts = [
            SimpleNamespace(name=name, data=SimpleNamespace(attributes={}))
            for name in names
        ]
        result = scope["select_components"](
            parts, validate(dict(TARGET, simple_iris=1))
        )
        self.assertEqual([p.name for p in parts], names[:2])
        self.assertEqual(removed, names[2:])
        self.assertEqual(repaired, names[:2])
        self.assertEqual(result["removed_components"], names[2:])

    def test_fresh_package_snapshot_hashes_and_no_final_restore(self):
        before = {p.name: p.read_bytes() for p in RECIPE.glob("*.py")}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("CPU packaging fixture; never execute")
            prefix = "assets/aster/general_face_eyes_test/"
            previous = root / prefix / "aster_head.glb"
            previous.parent.mkdir(parents=True)
            previous.write_bytes(b"previous output")
            request = build_request(TARGET, prefix, root)
            compile(request["code"], "<native request>", "exec")
            self.assertNotIn("head_recovery_617", request["code"])
            calls = [
                n.func.id
                for n in ast.walk(ast.parse(request["code"]))
                if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
            ]
            self.assertNotIn("measure_accent_connections", calls)
            enabled = build_request(dict(TARGET, supplemental_lashes=1), prefix, root)
            enabled_calls = [
                n.func.id
                for n in ast.walk(ast.parse(enabled["code"]))
                if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
            ]
            self.assertIn("measure_accent_connections", enabled_calls)
            self.assertIn("fit_evaluated_accent_roots", calls)
            self.assertNotIn("preserve_non_eye", calls)
            self.assertNotIn("refine_outer_eye", calls)
            self.assertIn("select_components(PARTS", request["code"])
            code = request["code"]
            self.assertLess(
                code.index('report["ear_root_fit"] = fit_ear_roots(shell)'),
                code.index("semantic_part_ordinals ="),
            )
            self.assertLess(
                code.index("semantic_part_ordinals ="),
                code.index("select_components(PARTS"),
            )
            self.assertIn("region = semantic_part_ordinals[part.name]", code)
            paths = {o["path"]: o["expectedSha256"] for o in request["outputs"]}
            self.assertIsNotNone(paths[prefix + "aster_head.glb"])
            self.assertIn(prefix + "input-manifest.json", paths)
            self.assertIn(prefix + "semantic_face_export.glb", paths)
            self.assertIn("/ (0.044 * 0.92) + 0.5", code)
            self.assertIn("iv = y / (0.036 * 0.92) + 0.5", code)
            self.assertLess(
                code.index("Recipe input changed:"),
                code.index("import copy, types, sys"),
            )
            verification = code.split("import copy, types, sys", 1)[0]
            exec(verification, {"beaver_input": lambda name: str(root / name)})
            (root / REQUIRED[0]).write_text("changed")
            with self.assertRaisesRegex(AssertionError, "Recipe input changed"):
                exec(verification, {"beaver_input": lambda name: str(root / name)})
            (root / REQUIRED[0]).unlink()
            with self.assertRaisesRegex(ValueError, "Missing"):
                build_request(TARGET, prefix, root)
        self.assertEqual(before, {p.name: p.read_bytes() for p in RECIPE.glob("*.py")})


if __name__ == "__main__":
    unittest.main()
