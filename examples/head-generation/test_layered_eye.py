"""Structure-first checks for annular iris and explicit recessed pupil."""

import json, math, unittest
from pathlib import Path
from test_style_math import pure_function

C = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]


class LayeredEyeTests(unittest.TestCase):
    def test_continuous_iris_has_one_boundary_and_no_hole(self):
        fn, _ = pure_function(
            "eye_socket.py",
            "annular_iris_mesh",
            {
                "C": C,
                "H": 1,
                "Y0": 0,
                "coord": lambda x, y, z: (x, y, z),
                "iris_depth": lambda x, y: 0,
            },
        )
        v, f = fn(1)
        n = C["iris_radial_samples"]
        r = C["iris_radial_rings"]
        self.assertEqual(len(v), 1 + r * n)
        self.assertEqual(len(f), r * n)
        uses = {}
        for face in f:
            self.assertIn(len(set(face)), [3, 4])
            q = [v[i] for i in face]
            area = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(q, q[1:] + q[:1])) / 2
            self.assertGreater(area, 0)
            for a, b in zip(face, face[1:] + face[:1]):
                e = tuple(sorted((a, b)))
                uses[e] = uses.get(e, 0) + 1
        self.assertEqual(sum(x == 1 for x in uses.values()), n)
        self.assertTrue(all(x <= 2 for x in uses.values()))

    def test_lower_aperture_rounding_preserves_upper_and_corners(self):
        fn, _ = pure_function("face.py", "eye_contour", {"C": C, "H": 1, "Y0": 0})
        old = dict(C, eye_lower_horizontal_power=C["eye_horizontal_power"])
        previous, _ = pure_function("face.py", "eye_contour", {"C": old, "H": 1, "Y0": 0})
        for t in [-1, 0, 1]:
            self.assertEqual(fn(1, t, False), previous(1, t, False))
        for t in [-0.8, -0.4, 0.4, 0.8]:
            self.assertGreater(fn(1, t, False)[1], previous(1, t, False)[1])
            self.assertEqual(fn(1, t, True), previous(1, t, True))

    def test_pupil_and_catchlight_are_separate_geometry(self):
        source = Path(__file__).with_name("eye_socket.py").read_text()
        self.assertIn("Pupil recessed sheet", source)
        self.assertIn("Catchlight white ellipse", source)
        self.assertLess(C["pupil_half_width"], C["iris_half_width"])
        self.assertLess(C["pupil_half_height"], C["iris_half_height"])
        self.assertTrue(-0.016 < C["pupil_recess"] < -0.005)


class WingWindingTests(unittest.TestCase):
    def test_original_lash_polygons_are_front_facing_and_not_self_crossing(self):
        style = json.loads(Path(__file__).with_name("face_style.json").read_text())
        fn, _ = pure_function("face.py", "lash_outline_points", {"C": C, "H": 1, "Y0": 0})

        def cross(a, b, c):
            return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])

        for side in [-1, 1]:
            for key in ["upper_lash_outline", "outer_lash_wing_outline"]:
                q = fn(side, style[key])
                edges = list(zip(q, q[1:] + q[:1]))
                self.assertGreater(sum(a[0] * b[1] - b[0] * a[1] for a, b in edges), 0)
                for i, (a, b) in enumerate(edges):
                    for j, (c, d) in enumerate(edges):
                        if abs(i - j) <= 1 or {i, j} == {0, len(edges) - 1}:
                            continue
                        self.assertFalse(
                            cross(a, b, c) * cross(a, b, d) < 0
                            and cross(c, d, a) * cross(c, d, b) < 0
                        )


class UpperLidOcclusionTests(unittest.TestCase):
    def test_lash_covers_inner_skin_lid_without_moving_aperture(self):
        style = json.loads(Path(__file__).with_name("face_style.json").read_text())
        self.assertLessEqual(C["upper_cover"], 0.011)
        self.assertGreater(style["upper_lash_thickness_m"], C["height_m"] * C["upper_cover"] * 0.8)
        self.assertAlmostEqual(C["eye_upper_arch"], 0.044)


class LashSurfacePlacementTests(unittest.TestCase):
    def test_upper_ribbon_is_a_coherent_thin_semantic_plane(self):
        fn, _ = pure_function("face.py", "upper_ink_depth", {"C": C, "H": 1, "Y0": 0, "Z0": 0})
        x = C["eye_center_x"]
        y = C["eye_corner_y"]
        self.assertAlmostEqual(fn(x, y), C["upper_ink_plane_depth"])
        self.assertAlmostEqual(fn(x + 0.02, y) - fn(x, y), 0.02 * C["upper_ink_plane_slope_x"])
        self.assertAlmostEqual(fn(-x, y), fn(x, y))
        self.assertAlmostEqual(fn(x, y + 0.02) - fn(x, y), 0.02 * C["upper_ink_plane_slope_y"])
