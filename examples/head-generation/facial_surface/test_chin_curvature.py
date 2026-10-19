"""Bound direct chin-height component without claiming final 3D displacement."""

import ast
import json
import math
from pathlib import Path
import unittest
from source_silhouette import apply_silhouette

RECIPE = Path(__file__).parents[1] / "checkpoints/current"


def underside(c):
    node = next(
        n
        for n in ast.parse((RECIPE / "shell.py").read_text()).body
        if isinstance(n, ast.FunctionDef) and n.name == "chin_underside_height"
    )
    env = {"C": c, "H": c["height_m"], "Y0": c["origin_y_m"], "sqrt": math.sqrt}
    exec(compile(ast.Module(body=[node], type_ignores=[]), "<chin>", "exec"), env)
    return env["chin_underside_height"]


class ChinTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())

    def test_default_exact(self):
        s, r = apply_silhouette(self.style, {"chin_softness": 0})
        self.assertEqual(s, self.style)
        self.assertIsNone(r)

    def test_only_return_lift_changes(self):
        s, r = apply_silhouette(self.style, {"chin_softness": 1})
        a = self.style["calibration"]
        b = s["calibration"]
        self.assertEqual({k for k in a if a[k] != b[k]}, {"chin_return_lift"})
        self.assertEqual(b["chin_return_lift"], 0.004)
        self.assertAlmostEqual(r["chin_direct_vertical_change_bound_m"], 0.000944)

    def test_four_rows_center_pinned_and_bound(self):
        s, r = apply_silhouette(self.style, {"chin_softness": 1})
        a = self.style["calibration"]
        b = s["calibration"]
        f = underside(a)
        g = underside(b)
        self.assertEqual(len(a["chin_underside_stations"]), 4)
        for _, lift, _ in a["chin_underside_stations"]:
            self.assertEqual(f(lift, 0), g(lift, 0))
            for i in range(-12, 13):
                q = i / 12
                delta = g(lift, q) - f(lift, q)
                self.assertLessEqual(delta, 1e-12)
                self.assertLessEqual(
                    abs(delta), r["chin_direct_vertical_change_bound_m"] + 1e-12
                )
                self.assertAlmostEqual(delta, -0.000944 * q * q)

    def test_front_shell_exponential_component(self):
        changed, report = apply_silhouette(self.style, {"chin_softness": 1})
        c = self.style["calibration"]
        d = changed["calibration"]
        tree = ast.parse((RECIPE / "shell.py").read_text())
        expression = next(
            n.value
            for n in ast.walk(tree)
            if isinstance(n, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == "yy" for t in n.targets)
        )
        code = compile(ast.Expression(body=expression), "<front height>", "eval")
        for y in (1.544, 1.55, 1.57, 1.614, 1.7):
            for q in (0, 0.25, 0.5, 1):

                def value(calibration):
                    return eval(
                        code,
                        {
                            "y": y,
                            "q": q,
                            "H": c["height_m"],
                            "Y0": c["origin_y_m"],
                            "C": calibration,
                            "exp": math.exp,
                            "upper_boundary_offset": lambda y, q: 0,
                        },
                    )

                delta = value(d) - value(c)
                self.assertAlmostEqual(
                    delta, -0.000944 * q * q * math.exp(-(y - 1.544) / 0.008)
                )
                self.assertLessEqual(
                    abs(delta), report["chin_direct_vertical_change_bound_m"] + 1e-12
                )

    def test_invalid_or_changed_source(self):
        for value in (True, -0.1, 1.1, float("nan"), "1"):
            with self.assertRaises(ValueError):
                apply_silhouette(self.style, {"chin_softness": value})
        self.style["calibration"]["chin_return_lift"] = 0.01
        with self.assertRaises(ValueError):
            apply_silhouette(self.style, {"chin_softness": 1})


if __name__ == "__main__":
    unittest.main()
