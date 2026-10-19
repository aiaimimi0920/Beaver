"""Lower-only shoulder warp contracts; native visual acceptance is separate."""

import ast
import importlib.util
import json
import math
from pathlib import Path
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate, adapt_lower_clearance
from contour_warp import lower_coordinate, lower_shape
from source_upper_warp import adapt_face_source, adapt_upper_sources, HELPER_NAMES
from blink_runtime import closure_delta

spec = importlib.util.spec_from_file_location(
    "lower_shoulder_intervals",
    Path(__file__).resolve().parents[1] / "facial_surface/interval_bounds.py",
)
ib = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ib)


class LowerShoulderTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.p = {"corner_slope_offset": -self.style["calibration"]["eye_corner_slope"]}
        self.base = adapt_style(self.style, self.p)["calibration"]

    def contour(self, c, enabled):
        src = adapt_face_source((RECIPE / "face.py").read_text(), enabled)
        nodes = [
            n
            for n in ast.parse(src).body
            if isinstance(n, ast.FunctionDef)
            and n.name in HELPER_NAMES | {"eye_contour"}
        ]
        env = dict(C=c, H=c["height_m"], Y0=c["origin_y_m"])
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "<contour>", "exec"), env)
        return env["eye_contour"]

    def test_default_and_invalid(self):
        self.assertEqual(adapt_style(self.style, {"lower_shoulder": 0}), self.style)
        for a in [-0.001, 1.001, True, float("nan")]:
            with self.assertRaises(ValueError):
                validate({"lower_shoulder": a})
        with self.assertRaises(ValueError):
            adapt_style(self.style, {"lower_shoulder": 1})

    def test_monotone_identity_and_interval(self):
        for a in [0, 0.25, 0.5, 1]:
            c = dict(self.base, eye_lower_shoulder=a)
            last = -3
            for i in range(1001):
                t = -2 + i / 250
                q = lower_coordinate(t, c)
                self.assertGreater(q, last)
                last = q
                if abs(t) >= 1 or t == 0:
                    self.assertEqual(q, t)
                b = ib.lower_coordinate_interval(ib.I(t, t + 0.002), c)
                for u in [t, t + 0.0005, t + 0.001, t + 0.0015, t + 0.002]:
                    self.assertLessEqual(b.lo, lower_coordinate(u, c))
                    self.assertGreaterEqual(b.hi, lower_coordinate(u, c))

    def test_actual_geometry_upper_fixed_lower_blink(self):
        old = self.contour(self.base, False)
        for a in [0.25, 0.5, 1]:
            c = adapt_style(self.style, dict(self.p, lower_shoulder=a))["calibration"]
            new = self.contour(c, True)
            for side in [-1, 1]:
                for i in range(101):
                    t = -1 + i / 50
                    self.assertEqual(new(side, t, True), old(side, t, True))
                    x, y = new(side, t, False)
                    self.assertEqual(x, old(side, t, False)[0])
                    self.assertGreaterEqual(y + 1e-12, old(side, t, False)[1])
                    if i in [0, 50, 100]:
                        self.assertEqual(y, old(side, t, False)[1])
                    if 0 < i < 100:
                        seam = (
                            c["origin_y_m"]
                            + c["height_m"] * c["eye_corner_y"]
                            - 0.0015 * (1 - t * t)
                        )
                        self.assertAlmostEqual(
                            y + closure_delta(x, y, "lower", c), seam
                        )

    def test_lower_horizontal_profile(self):
        for scale in [1, 1.2, 1.5]:
            c = adapt_style(self.style, dict(self.p, lower_horizontal_roundness=scale))[
                "calibration"
            ]
            self.assertEqual(
                c["eye_lower_horizontal_power"],
                self.base["eye_lower_horizontal_power"] * scale,
            )
            old = self.contour(self.base, False)
            new = self.contour(c, False)
            for side in [-1, 1]:
                for t in [-1, -0.75, -0.5, 0, 0.5, 0.75, 1]:
                    self.assertEqual(new(side, t, True), old(side, t, True))
                    self.assertEqual(new(side, t, False)[0], old(side, t, False)[0])
                    self.assertLessEqual(
                        new(side, t, False)[1], old(side, t, False)[1] + 1e-12
                    )
                    if t in [-1, 0, 1]:
                        self.assertEqual(new(side, t, False), old(side, t, False))
        for value in [0.99, 1.51, True, float("nan")]:
            with self.assertRaises(ValueError):
                validate({"lower_horizontal_roundness": value})

    def test_pocket_radius_and_source_contract(self):
        sources = {
            n: (RECIPE / n).read_text()
            for n in ["face.py", "eye_socket.py", "lateral_fit.py"]
        }
        sources["eye_socket.py"] = adapt_lower_clearance(sources["eye_socket.py"], True)
        src = adapt_upper_sources(sources, True)
        node = next(
            n.value
            for n in ast.walk(ast.parse(src["eye_socket.py"]))
            if isinstance(n, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == "radius" for t in n.targets)
        )
        expression = compile(ast.Expression(node), "<radius>", "eval")
        for a in [0.25, 0.5, 1]:
            c = adapt_style(self.style, dict(self.p, lower_shoulder=a))["calibration"]
            H = c["height_m"]
            arch = c["eye_lower_arch"]
            power = c["eye_lower_power"]
            for t in [-0.95, -0.7, -0.3, 0, 0.3, 0.7, 0.95]:
                y = -H * arch * lower_shape(t, c)
                env = dict(
                    C=c,
                    H=H,
                    cy=0,
                    arch=arch,
                    power=power,
                    t=t,
                    y=y,
                    sqrt=math.sqrt,
                    abs=abs,
                    lower_coordinate=lower_coordinate,
                )
                self.assertAlmostEqual(eval(expression, {"__builtins__": {}}, env), 1)
        with self.assertRaises(ValueError):
            adapt_face_source("changed", True)


class LowerHorizontalCombinedTests(unittest.TestCase):
    contour = LowerShoulderTests.contour
    test_actual_geometry_upper_fixed_lower_blink = (
        LowerShoulderTests.test_actual_geometry_upper_fixed_lower_blink
    )
    test_pocket_radius_and_source_contract = (
        LowerShoulderTests.test_pocket_radius_and_source_contract
    )
    test_monotone_identity_and_interval = (
        LowerShoulderTests.test_monotone_identity_and_interval
    )

    def setUp(self):
        LowerShoulderTests.setUp(self)
        self.p["lower_horizontal_roundness"] = 1.5
        self.base = adapt_style(self.style, self.p)["calibration"]


if __name__ == "__main__":
    unittest.main()
