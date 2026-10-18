"""Layer selection preserves the reference-like groups and their full checks."""

import ast
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from parameters import validate, adapt_style
from bundle import build_request, REQUIRED, RECIPE


class LashLayerTests(unittest.TestCase):
    def controls(self, enabled):
        return dict(
            short_upper_tuft=1,
            supplemental_lashes=0,
            lash_contour_follow=1,
            layered_lashes=enabled,
            lid_fold=0,
            simple_iris=1,
        )

    def test_upper_layer_repositions_only_its_own_outline(self):
        original = json.loads((RECIPE / "face_style.json").read_text())
        plain = adapt_style(original, self.controls(1))
        added = adapt_style(original, dict(self.controls(1), upper_lash_layer=1))
        changed = {key for key in plain if plain[key] != added[key]}
        self.assertEqual(changed, {"inner_lash_fork_outline"})
        stations = [row[0] for row in added["inner_lash_fork_outline"]]
        self.assertEqual(min(stations), 0.24)
        self.assertEqual(max(stations), 0.68)
        outline = added["inner_lash_fork_outline"]
        self.assertTrue(all(a[1] < b[1] for a, b in zip(outline[:3], outline[1:4])))
        self.assertLess(outline[4][1], outline[3][1])
        with self.assertRaises(ValueError):
            validate(dict(self.controls(0), upper_lash_layer=1))
        with self.assertRaises(ValueError):
            validate(dict(self.controls(1), upper_lash_layer=0.5))

    def test_layered_lower_accents_have_distinct_visual_roles(self):
        original = json.loads((RECIPE / "face_style.json").read_text())
        before = json.dumps(original, sort_keys=True)
        plain = adapt_style(original, self.controls(0))
        layered = adapt_style(original, self.controls(1))
        self.assertEqual(json.dumps(original, sort_keys=True), before)
        self.assertEqual(plain["accent_lower_tip_t"], original["accent_lower_tip_t"])
        self.assertEqual(layered["accent_lower_tip_t"], 0.40)
        self.assertEqual(
            layered["lateral_accent_profile"][3], original["lateral_accent_profile"][3]
        )
        self.assertAlmostEqual(
            layered["lateral_accent_profile"][6][1],
            original["lateral_accent_profile"][6][1] * 0.45,
        )
        self.assertEqual(
            layered["accent_lower_tip_drop_H"], original["accent_lower_tip_drop_H"]
        )

    def test_selection_counts_each_retained_group_on_both_sides(self):
        tree = ast.parse(Path(__file__).with_name("runtime.py").read_text())
        tree.body = [
            n for n in tree.body if not isinstance(n, (ast.Import, ast.ImportFrom))
        ]
        removed = []
        scope = {
            "bpy": SimpleNamespace(
                data=SimpleNamespace(
                    objects=SimpleNamespace(
                        remove=lambda obj, do_unlink: removed.append(obj.name)
                    )
                )
            ),
            "store_mouth_deltas": lambda obj: None,
        }
        exec(compile(tree, "mock selection", "exec"), scope)
        names = []
        for side in (-1, 1):
            names += [
                prefix + str(side)
                for prefix in (
                    "Upper lash ",
                    "Inner forked lash blade ",
                    "Upper swept lash tuft ",
                    "Attached outer lash wing ",
                    "Outer lateral eye liner ",
                    "Outer companion lash ",
                    "Lower outer lash tuft ",
                    "Embedded lateral child 0 ",
                    "Embedded lateral child 1 ",
                    "Tapered upper eyelid fold ",
                    "Pupil separate accent ",
                )
            ]
        for enabled, extra, expected in [(0, 0, 2), (1, 0, 14), (1, 1, 16)]:
            parts = [
                SimpleNamespace(name=n, data=SimpleNamespace(attributes={}))
                for n in names
            ]
            report = scope["select_components"](
                parts, validate(dict(self.controls(enabled), upper_lash_layer=extra))
            )
            self.assertEqual(len(report["retained_lash_components"]), expected)
            self.assertTrue(
                all(
                    not p.name.startswith(("Tapered upper eyelid", "Pupil separate"))
                    for p in parts
                )
            )
            self.assertEqual(len(parts), expected + 2)
            if enabled:
                wanted = {
                    n
                    for n in names
                    if not n.startswith(
                        (
                            "Upper lash ",
                            "Inner forked lash blade ",
                            "Tapered upper eyelid fold ",
                            "Pupil separate accent ",
                        )
                    )
                }
                if extra:
                    wanted.update(
                        n for n in names if n.startswith("Inner forked lash blade ")
                    )
                self.assertEqual(set(report["retained_lash_components"]), wanted)

    def test_layered_package_keeps_full_connection_check(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                p = root / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("CPU fixture, never execute")
            request = build_request(
                dict(self.controls(1), upper_lash_layer=1),
                "assets/aster/general_face_eyes_test/",
                root,
                blink=True,
            )
            self.assertIn("measure_accent_connections()", request["code"])
            tree = ast.parse(request["code"])
            calls = [
                n
                for n in ast.walk(tree)
                if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
            ]
            self.assertTrue(
                any(n.func.id == "measure_accent_connections" for n in calls)
            )
            self.assertFalse(
                any(n.func.id == "validate_retained_tuft_roots" for n in calls)
            )
            self.assertIn(
                "Intermediate layer attachment exceeds design bound", request["code"]
            )
            self.assertIn("Intermediate upper layer root disconnected", request["code"])
            compile(request["code"], "layered request", "exec")

    def test_invalid_controls(self):
        for value in (0.5, True, -1, 2):
            with self.assertRaises(ValueError):
                validate(dict(self.controls(0), layered_lashes=value))
        with self.assertRaises(ValueError):
            validate({"layered_lashes": 1})
