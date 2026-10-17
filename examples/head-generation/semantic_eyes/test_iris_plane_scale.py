"""CPU evaluation of production ocular geometry for the independent plane control."""

import ast
import copy
import json
import math
from pathlib import Path
from types import SimpleNamespace
import unittest

from parameters import adapt_style, validate


class IrisPlaneScaleTests(unittest.TestCase):
    def setUp(self):
        self.recipe = Path(__file__).parents[1] / "checkpoints/current"
        self.base = json.loads((self.recipe / "face_style.json").read_text())

    def test_identity_bounds_and_independent_calibration(self):
        self.assertEqual(adapt_style(self.base, {}), self.base)
        self.assertEqual(
            adapt_style(self.base, {"iris_plane_slope_scale": 1}), self.base
        )
        for bad in (0.84999, 1.00001, True, float("inf"), float("nan")):
            with self.assertRaises(ValueError):
                validate({"iris_plane_slope_scale": bad})
        for scale in (0.85, 0.9, 1):
            for offset in (0, -0.006):
                request = {"iris_horizontal_offset": offset, "iris_width": 1.08}
                before = adapt_style(self.base, request)
                after = adapt_style(
                    self.base, dict(request, iris_plane_slope_scale=scale)
                )
                self.assertEqual(
                    after["calibration"]["iris_plane_slope"],
                    self.base["calibration"]["iris_plane_slope"] * scale,
                )
                after["calibration"]["iris_plane_slope"] = before["calibration"][
                    "iris_plane_slope"
                ]
                self.assertEqual(after, before)

    def geometry(self, calibration, side):
        names = {
            "iris_depth",
            "iris_grid_point",
            "annular_iris_mesh",
            "build_pupil_layer",
            "build_catchlight",
        }
        nodes = [
            node
            for node in ast.parse((self.recipe / "eye_socket.py").read_text()).body
            if isinstance(node, ast.FunctionDef) and node.name in names
        ]
        captured = []

        def mesh(name, vertices, faces, color):
            captured.append((name, vertices))

            class Mesh(dict):
                data = SimpleNamespace(polygons=[SimpleNamespace(area=1)])

            return Mesh()

        env = dict(
            C=calibration,
            H=calibration["height_m"],
            Y0=calibration["origin_y_m"],
            Z0=0,
            STYLE=self.base,
            coord=lambda *xyz: xyz,
            mesh=mesh,
            grid_faces=lambda *args: [],
            sqrt=math.sqrt,
            sin=math.sin,
            cos=math.cos,
            pi=math.pi,
        )
        exec(
            compile(ast.Module(body=nodes, type_ignores=[]), "ocular-source", "exec"),
            env,
        )
        captured.append(("iris", env["annular_iris_mesh"](side)[0]))
        env["build_pupil_layer"](side, False)
        env["build_pupil_layer"](side, True)
        env["build_catchlight"](side)
        return captured

    def test_actual_iris_pupil_accent_catchlight_share_pivot_and_depth_change(self):
        c = self.base["calibration"]
        altered = adapt_style(self.base, {"iris_plane_slope_scale": 0.85})[
            "calibration"
        ]
        snapshot = copy.deepcopy(c)
        for side in (-1, 1):
            before, after = self.geometry(c, side), self.geometry(altered, side)
            self.assertEqual(len(before), 4)
            for (name, old), (new_name, new) in zip(before, after):
                self.assertEqual(name, new_name)
                self.assertEqual(len(old), len(new))
                for a, b in zip(old, new):
                    self.assertEqual(a[:2], b[:2])
                    dx = abs(a[0]) - c["height_m"] * c["iris_center_x"]
                    self.assertAlmostEqual(
                        b[2] - a[2],
                        (altered["iris_plane_slope"] - c["iris_plane_slope"]) * dx,
                        places=14,
                    )
            self.assertEqual(before[0][1][0], after[0][1][0])
        self.assertEqual(c, snapshot)


if __name__ == "__main__":
    unittest.main()
