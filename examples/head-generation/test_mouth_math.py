"""Pure geometry regression checks; no Blender or model generation."""

import ast
import json
import math
from pathlib import Path
import unittest

ROOT = Path(__file__).parent


def load_math(half_gap=None):
    style = json.loads((ROOT / "face_style.json").read_text())
    calibration = style["calibration"]
    if half_gap is not None:
        calibration["mouth_rest_half_gap"] = half_gap
    scope = dict(
        C=calibration,
        H=calibration["height_m"],
        Y0=calibration["origin_y_m"],
        Vector=tuple,
        sin=math.sin,
        cos=math.cos,
        exp=math.exp,
    )
    tree = ast.parse((ROOT / "mouth.py").read_text())
    allowed = {"MOUTH_Y", "REST_GAP", "rest_seam_y", "mouth_half_contour", "jaw_delta"}
    nodes = [
        node
        for node in tree.body
        if (isinstance(node, ast.FunctionDef) and node.name in allowed)
        or (
            isinstance(node, ast.Assign)
            and isinstance(node.targets[0], ast.Name)
            and node.targets[0].id in allowed
        )
    ]
    exec(compile(ast.Module(body=nodes, type_ignores=[]), "<mouth_math>", "exec"), scope)
    return scope


class MouthMathTests(unittest.TestCase):
    def test_oral_profile_keeps_attachment_and_expands_rear(self):
        c = json.loads((ROOT / "face_style.json").read_text())["calibration"]
        rings = c["oral_rings"]
        self.assertEqual(rings[0], [0, 1, 0, 0])
        self.assertGreaterEqual(len(rings), 4)
        self.assertTrue(all(a[0] < b[0] for a, b in zip(rings, rings[1:])))
        self.assertGreater(rings[-1][1], 1)
        self.assertGreater(rings[-1][2] + rings[-1][3], 0.15)
        self.assertTrue(all(r[1] > 0 and r[2] >= 0 and r[3] >= 0 for r in rings))

    def test_aperture_and_cavity_share_curved_seam(self):
        scope = load_math()
        for i in range(25):
            angle = -math.pi / 2 + math.pi * i / 24
            x, y = scope["mouth_half_contour"](angle)
            normalized = (y - scope["rest_seam_y"](x)) / scope["REST_GAP"]
            self.assertAlmostEqual(normalized, math.sin(angle), places=10)

    def test_raised_corner_lower_lip_moves_down(self):
        scope = load_math()
        x, y = scope["mouth_half_contour"](-math.pi / 6)
        self.assertGreater(y, scope["MOUTH_Y"])
        delta = scope["jaw_delta"](x, y, 0)
        self.assertLess(delta[2], -0.010)
        self.assertGreater(delta[1], 0)

    def test_gap_change_preserves_lip_classification(self):
        for gap in (0.0004, 0.00065, 0.0015):
            scope = load_math(gap)
            for angle in (0.2, 0.5, 1.0):
                x, upper = scope["mouth_half_contour"](angle)
                _, lower = scope["mouth_half_contour"](-angle)
                du = scope["jaw_delta"](x, upper, 0)
                dl = scope["jaw_delta"](x, lower, 0)
                self.assertEqual(du[1], 0)
                self.assertGreater(dl[1], 0)
                self.assertGreater(upper + du[2], lower + dl[2])

    def test_symmetry_and_connected_corner(self):
        scope = load_math()
        x, y = scope["mouth_half_contour"](0)
        self.assertEqual(y, scope["rest_seam_y"](x))
        for fraction in (0, 0.3, 0.8, 1):
            x = 0.0195 * fraction
            y = scope["rest_seam_y"](x)
            self.assertEqual(scope["jaw_delta"](x, y, 0), scope["jaw_delta"](-x, y, 0))


if __name__ == "__main__":
    unittest.main()
