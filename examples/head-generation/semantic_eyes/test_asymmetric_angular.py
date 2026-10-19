"""Exact 807 combination: native contour consumers agree before generation."""

import ast
import json
from pathlib import Path
import unittest
from bundle import RECIPE
from parameters import adapt_style
from source_upper_warp import adapt_upper_sources, HELPER_NAMES, adapt_face_source
from contour_warp import upper_shape, lower_shape
from blink_runtime import closure_delta


class AsymmetricAngularTests(unittest.TestCase):
    def test_real_contour_inverse_and_closed_seam(self):
        request = json.loads(
            (
                Path(__file__).parents[1]
                / "eye_presets"
                / "asymmetric-angular-807.json"
            ).read_text()
        )["eye_parameters"]
        c = adapt_style(json.loads((RECIPE / "face_style.json").read_text()), request)[
            "calibration"
        ]
        source = adapt_face_source((RECIPE / "face.py").read_text(), True)
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef)
            and n.name in HELPER_NAMES | {"eye_contour"}
        ]
        scope = dict(C=c, H=c["height_m"], Y0=c["origin_y_m"])
        exec(
            compile(
                ast.Module(body=nodes, type_ignores=[]), "<actual contour>", "exec"
            ),
            scope,
        )
        curve = scope["eye_contour"]
        sources = adapt_upper_sources(
            {
                n: (RECIPE / n).read_text()
                for n in ("face.py", "eye_socket.py", "lateral_fit.py")
            },
            True,
        )
        node = next(
            n
            for n in ast.parse(sources["lateral_fit.py"]).body
            if isinstance(n, ast.FunctionDef) and n.name == "eye_rim_x_at_height"
        )
        env = dict(upper_shape=upper_shape, lower_shape=lower_shape)
        exec(
            compile(
                ast.Module(body=[node], type_ignores=[]), "<actual inverse>", "exec"
            ),
            env,
        )
        for side in (-1, 1):
            for i in range(101):
                t = -1 + i / 50
                for upper in (True, False):
                    x, y = curve(side, t, upper)
                    seam = (
                        c["origin_y_m"]
                        + c["height_m"] * c["eye_corner_y"]
                        - 0.0015 * (1 - t * t)
                    )
                    self.assertAlmostEqual(
                        y + closure_delta(x, y, "upper" if upper else "lower", c), seam
                    )
        for upper in (True, False):
            previous = curve(1, 0.4, upper)[1]
            for i in range(1, 101):
                t = 0.4 + 0.6 * i / 100
                x, y = curve(1, t, upper)
                (
                    self.assertLess(y, previous)
                    if upper
                    else self.assertGreater(y, previous)
                )
                self.assertAlmostEqual(env["eye_rim_x_at_height"](y, c), x, places=11)
                previous = y
