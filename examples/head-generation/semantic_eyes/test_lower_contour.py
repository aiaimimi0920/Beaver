"""Independent lower-lid shape control, using the actual inherited contour."""

import ast
import json
import math
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate
from blink_runtime import closure_delta


def contour(calibration):
    tree = ast.parse((RECIPE / "face.py").read_text())
    node = next(
        n
        for n in tree.body
        if isinstance(n, ast.FunctionDef) and n.name == "eye_contour"
    )
    scope = {
        "C": calibration,
        "H": calibration["height_m"],
        "Y0": calibration["origin_y_m"],
    }
    exec(
        compile(ast.Module(body=[node], type_ignores=[]), "<actual contour>", "exec"),
        scope,
    )
    return scope["eye_contour"]


class LowerContourTests(unittest.TestCase):
    def setUp(self):
        self.original = json.loads((RECIPE / "face_style.json").read_text())

    def test_default_is_exact_identity_and_limits_fail_closed(self):
        self.assertEqual(
            adapt_style(self.original, {"lower_roundness": 1}), self.original
        )
        for value in (0.99, 1.251, float("nan"), True):
            with self.assertRaises(ValueError):
                validate({"lower_roundness": value})

    def test_only_lower_power_changes_in_calibration(self):
        altered = adapt_style(self.original, {"lower_roundness": 1.25})
        old, new = self.original["calibration"], altered["calibration"]
        changed = [key for key in old if old[key] != new[key]]
        self.assertEqual(changed, ["eye_lower_power"])
        self.assertEqual(new["eye_lower_power"], old["eye_lower_power"] * 1.25)

    def test_real_contour_preserves_upper_and_corners_and_shallows_shoulders(self):
        base = contour(self.original["calibration"])
        for factor in (1, 1.125, 1.25):
            c = adapt_style(self.original, {"lower_roundness": factor})["calibration"]
            new = contour(c)
            for side in (-1, 1):
                for i in range(101):
                    t = -1 + i / 50
                    old_x, old_upper = base(side, t, True)
                    _, old_lower = base(side, t, False)
                    x, upper = new(side, t, True)
                    _, lower = new(side, t, False)
                    self.assertEqual(x, old_x)
                    self.assertEqual(upper, old_upper)
                    self.assertGreaterEqual(lower + 1e-12, old_lower)
                    self.assertLessEqual(lower, upper)
                    self.assertTrue(all(math.isfinite(v) for v in (x, upper, lower)))
                    if i in (0, 50, 100):
                        self.assertEqual(lower, old_lower)
                    if 0 < i < 100:
                        seam = (
                            c["origin_y_m"]
                            + c["height_m"]
                            * (c["eye_corner_y"] + c["eye_corner_slope"] * t)
                            - 0.0015 * (1 - t * t)
                        )
                        self.assertAlmostEqual(
                            lower + closure_delta(x, lower, "lower", c), seam
                        )

    def test_clearance_compatibility_switch_and_threshold_are_explicit(self):
        from parameters import adapt_lower_clearance

        source = (RECIPE / "eye_socket.py").read_text()

        def radius(text, y):
            tree = ast.parse(text)
            node = next(
                n.value
                for n in ast.walk(tree)
                if isinstance(n, ast.Assign)
                and any(isinstance(t, ast.Name) and t.id == "radius" for t in n.targets)
            )
            c = self.original["calibration"]
            env = dict(
                C=c, H=1, cy=0, arch=1, power=0.53, t=0.7, y=y, sqrt=math.sqrt, abs=abs
            )
            return eval(
                compile(ast.Expression(node), "<radius>", "eval"),
                {"__builtins__": {}},
                env,
            )

        fixed = adapt_lower_clearance(source, True)
        self.assertGreater(radius(source, -0.65), 1.01)
        self.assertLess(radius(fixed, -0.65), 1.01)
        self.assertGreater(radius(fixed, -0.8), 1.01)
        self.assertEqual(radius(source, 0.65), radius(fixed, 0.65))

    def test_clearance_radius_tracks_modified_lower_curve(self):
        from parameters import adapt_lower_clearance

        source = (RECIPE / "eye_socket.py").read_text()
        self.assertEqual(adapt_lower_clearance(source, False), source)
        tree = ast.parse(adapt_lower_clearance(source, True))
        node = next(
            n.value
            for n in ast.walk(tree)
            if isinstance(n, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == "radius" for t in n.targets)
        )
        expression = compile(ast.Expression(node), "<actual clearance radius>", "eval")
        for factor in (1.125, 1.25):
            c = adapt_style(self.original, {"lower_roundness": factor})["calibration"]
            H, cy = c["height_m"], 0
            arch, power = c["eye_lower_arch"], c["eye_lower_power"]
            for t in (-0.95, -0.7, -0.25, 0, 0.4, 0.9):
                y = -H * arch * (1 - abs(t) ** c["eye_lower_horizontal_power"]) ** power
                env = dict(
                    C=c,
                    H=H,
                    cy=cy,
                    arch=arch,
                    power=power,
                    t=t,
                    y=y,
                    sqrt=math.sqrt,
                    abs=abs,
                )
                self.assertAlmostEqual(eval(expression, {"__builtins__": {}}, env), 1)
        with self.assertRaises(ValueError):
            adapt_lower_clearance("changed source", True)


if __name__ == "__main__":
    unittest.main()
