"""Pure mathematical checks; no Blender import or asset generation."""

import ast
import copy
import json
import math
import unittest
from pathlib import Path

ROOT = Path(__file__).parent
STYLE = json.loads((ROOT / "face_style.json").read_text())


def pure_function(module, name, extra):
    tree = ast.parse((ROOT / module).read_text())
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == name]
    scope = {key: getattr(math, key) for key in ["sin", "cos", "pi", "exp", "sqrt", "atan2"]}
    scope.update(extra)
    exec(compile(ast.Module(body=nodes, type_ignores=[]), "<pure style math>", "exec"), scope)
    return scope[name], scope


class StyleMathTests(unittest.TestCase):
    def test_top_boundary_depth_reaches_explicit_profile(self):
        c = STYLE["calibration"]
        curve_fn, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "upper_boundary_depth",
            {
                "C": c,
                "H": c["height_m"],
                "Y0": c["origin_y_m"],
                "Z0": c["chin_depth_m"],
                "curve": curve_fn,
                "depth": lambda x, y: 0.08,
                "width": lambda y: 0.1,
            },
        )
        for q in [0, 0.3, 0.65, 0.9, 1]:
            self.assertAlmostEqual(fn(0, 0, c["origin_y_m"], q), 0.08)
            self.assertAlmostEqual(
                fn(0, 0, c["origin_y_m"] + c["height_m"], q),
                c["chin_depth_m"] + c["height_m"] * curve_fn(q, 1, c["top_boundary_depth_profile"]),
            )

    def test_upper_boundary_warp_preserves_row_order(self):
        c = STYLE["calibration"]
        curve_fn, _ = pure_function("face.py", "curve", {})
        offset, _ = pure_function(
            "shell.py",
            "upper_boundary_offset",
            {"C": c, "H": c["height_m"], "Y0": c["origin_y_m"], "curve": curve_fn},
        )
        for q in [i / 30 for i in range(31)]:
            ys = [c["origin_y_m"] + c["height_m"] * i / 200 for i in range(201)]
            moved = [y - offset(y, q) for y in ys]
            self.assertTrue(all(b > a for a, b in zip(moved, moved[1:])))
        self.assertAlmostEqual(offset(c["origin_y_m"] + c["height_m"], 0), 0)

    def test_rear_cavity_sections_are_simple(self):
        c = STYLE["calibration"]
        self.assertEqual(c["pocket_ring_samples"], 32)
        self.assertGreater(c["pocket_body_length"], 0)
        self.assertEqual(len(c["oral_rings"]), 4)
        # The new brief requires a gently contracting back wall, not a tube.
        for rear, middle in zip(c["oral_rings"][-1][1:], c["oral_rings"][-2][1:]):
            self.assertGreater(rear, 0.8 * middle)
            self.assertLess(rear, middle)
        self.assertTrue(all(a[0] < b[0] for a, b in zip(c["oral_rings"], c["oral_rings"][1:])))

    def test_iris_grid_has_no_collapsed_poles(self):
        fn, _ = pure_function("eye_socket.py", "iris_grid_point", {})
        n = STYLE["calibration"]["iris_grid_resolution"]
        self.assertEqual(n, 9)
        points = [
            [fn(-1 + 2 * i / (n - 1), -1 + 2 * j / (n - 1)) for i in range(n)] for j in range(n)
        ]
        shortest = 10
        for j in range(n):
            for i in range(n):
                x, y = points[j][i]
                self.assertTrue(math.isfinite(x) and math.isfinite(y))
                if i in [0, n - 1] or j in [0, n - 1]:
                    self.assertAlmostEqual(x * x + y * y, 1, places=10)
                for jj, ii in [(j + 1, i), (j, i + 1)]:
                    if jj < n and ii < n:
                        shortest = min(shortest, math.dist(points[j][i], points[jj][ii]))
                if i < n - 1 and j < n - 1:
                    q = [points[j][i], points[j][i + 1], points[j + 1][i + 1], points[j + 1][i]]
                    area = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(q, q[1:] + q[:1])) / 2
                    self.assertGreater(area, 0.01)
        self.assertGreater(shortest, 0.16)

    def test_eye_corners_join_and_lower_lid_covers_iris_base(self):
        c = STYLE["calibration"]
        h = c["height_m"]
        y0 = c["origin_y_m"]
        fn, _ = pure_function("face.py", "eye_contour", {"C": c, "H": h, "Y0": y0})
        for side in [-1, 1]:
            for t in [-1, 1]:
                self.assertEqual(fn(side, t, True), fn(side, t, False))
            for t in [-0.8, -0.4, 0, 0.4, 0.8]:
                self.assertGreater(fn(side, t, True)[1], fn(side, t, False)[1])
        iris_bottom = y0 + h * (c["iris_center_y"] - c["iris_half_height"])
        lower_cover = fn(1, 0, False)[1] + h * c["lower_cover"]
        self.assertGreater(lower_cover, iris_bottom)
        self.assertLess(lower_cover - iris_bottom, 0.02 * h)

    def test_ear_membrane_root_is_exact_and_outward(self):
        curve_fn, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("ears.py", "membrane_offset", {"curve": curve_fn})
        c = STYLE["calibration"]
        for t in [i / 20 for i in range(21)]:
            self.assertEqual(fn(t, 0, c, -0.33), (0, 0))
            xs = [fn(t, u, c, -0.33)[0] for u in [0, 0.3, 0.7, 1]]
            self.assertTrue(all(b >= a for a, b in zip(xs, xs[1:])))
            for u in [0, 0.3, 0.7, 1]:
                self.assertTrue(all(math.isfinite(v) for v in fn(t, u, c, -0.33)))

    def test_iris_palette_is_finite_and_bounded(self):
        fn, _ = pure_function("textures.py", "iris_color", {"STYLE": STYLE})
        for iy in range(17):
            for ix in range(17):
                rgba = fn(ix / 16, iy / 16)
                self.assertEqual(rgba[3], 1)
                self.assertTrue(all(math.isfinite(c) and 0 <= c <= 1 for c in rgba))

    def test_ear_palette_is_bounded_without_reference_pixels(self):
        fn, _ = pure_function(
            "textures.py", "ear_color", {"STYLE": STYLE, "SKIN": STYLE["skin_rgb"]}
        )
        for iy in range(21):
            for ix in range(21):
                rgba = fn(ix / 20, iy / 20)
                self.assertEqual(rgba[3], 1)
                self.assertTrue(all(math.isfinite(c) and 0 <= c <= 1 for c in rgba))

    def test_ear_membrane_profile_has_bounded_depth(self):
        curve_fn, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("ears.py", "membrane_offset", {"curve": curve_fn})
        c = STYLE["calibration"]
        for t in [i / 30 for i in range(31)]:
            for u in [i / 10 for i in range(11)]:
                outward, posterior = fn(t, u, c, -0.33)
                self.assertTrue(0 <= outward < 0.13)
                self.assertTrue(0 <= posterior <= 0.15)

    def test_nasal_profiles_are_monotone_and_bounded(self):
        curve_fn, _ = pure_function("face.py", "curve", {})
        factor, _ = pure_function(
            "face.py", "nose_profile_factor", {"C": STYLE["calibration"], "curve": curve_fn}
        )
        for height in [0.20, 0.25, 0.265, 0.28, 0.34, 0.4]:
            values = [factor(i / 100, height) for i in range(101)]
            self.assertAlmostEqual(values[0], 1)
            self.assertAlmostEqual(values[-1], 0)
            self.assertTrue(all(0 <= x <= 1 for x in values))
            self.assertTrue(all(a >= b - 1e-10 for a, b in zip(values, values[1:])))

    def test_basal_fullness_fades_above_tip(self):
        curve_fn, _ = pure_function("face.py", "curve", {})
        factor, _ = pure_function(
            "face.py", "nose_profile_factor", {"C": STYLE["calibration"], "curve": curve_fn}
        )
        self.assertGreater(factor(0.4, 0.25), factor(0.4, 0.28))
        self.assertAlmostEqual(factor(0.4, 0.28), factor(0.4, 0.4))


if __name__ == "__main__":
    unittest.main()
