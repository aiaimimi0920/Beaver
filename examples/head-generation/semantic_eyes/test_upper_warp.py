"""Cross-consumer contracts for the opt-in upper-only warp."""

import ast
import json
import math
from pathlib import Path
import sys
import unittest
import tempfile
from bundle import RECIPE, REQUIRED, build_request
from parameters import adapt_style, validate, upper_change
from contour_warp import (
    eye_warp,
    eye_warp_derivative,
    upper_shape,
    upper_shape_derivative,
)
from source_upper_warp import adapt_face_source, adapt_upper_sources, HELPER_NAMES
from blink_runtime import closure_delta


class UpperWarpTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.p = {"corner_slope_offset": -self.style["calibration"]["eye_corner_slope"]}
        self.base = adapt_style(self.style, self.p)["calibration"]

    def contour(self, c, enabled):
        source = adapt_face_source((RECIPE / "face.py").read_text(), enabled)
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef)
            and n.name in HELPER_NAMES | {"eye_contour"}
        ]
        env = dict(C=c, H=c["height_m"], Y0=c["origin_y_m"])
        exec(
            compile(
                ast.Module(body=nodes, type_ignores=[]), "<actual contour>", "exec"
            ),
            env,
        )
        return env["eye_contour"]

    def test_identity_and_fail_closed_range(self):
        source = (RECIPE / "face.py").read_text()
        self.assertEqual(adapt_face_source(source, False), source)
        self.assertEqual(adapt_style(self.style, {"upper_asymmetry": 0}), self.style)
        for value in (-0.10001, 0.10001, True, float("nan")):
            with self.assertRaises(ValueError):
                validate({"upper_asymmetry": value})
        with self.assertRaisesRegex(ValueError, "level eye corners"):
            adapt_style(self.style, {"upper_asymmetry": 0.05})
        with self.assertRaises(ValueError):
            adapt_face_source("changed", True)

    def test_monotonicity_endpoints_extension_and_derivative(self):
        for a in (-0.1, -0.03, 0, 0.03, 0.1):
            c = dict(self.base, eye_upper_asymmetry=a)
            last = -2
            for i in range(1001):
                t = -1 + i / 500
                value = eye_warp(t, c)
                self.assertGreater(value, last)
                self.assertGreaterEqual(eye_warp_derivative(t, c), 0.828)
                last = value
                if abs(t) < 0.99 and abs(eye_warp(t, c)) > 0.02:
                    numeric = (
                        upper_shape(t + 1e-6, c) - upper_shape(t - 1e-6, c)
                    ) / 2e-6
                    self.assertAlmostEqual(
                        upper_shape_derivative(t, c), numeric, places=5
                    )
            for t in (-2, -1, 1, 2):
                self.assertEqual(eye_warp(t, c), t)
                self.assertEqual(eye_warp_derivative(t, c), 1)

    def test_actual_contour_lower_layout_and_blink_are_coupled(self):
        old = self.contour(self.base, False)
        for a in (-0.1, 0.1):
            p = dict(self.p, upper_asymmetry=a)
            c = adapt_style(self.style, p)["calibration"]
            new = self.contour(c, True)
            for side in (-1, 1):
                for i in range(101):
                    t = -1 + i / 50
                    self.assertEqual(new(side, t, False), old(side, t, False))
                    x, y = new(side, t, True)
                    self.assertEqual(x, old(side, t, True)[0])
                    if i in (0, 100):
                        self.assertEqual(y, old(side, t, True)[1])
                    if 0 < i < 100:
                        seam = (
                            c["origin_y_m"]
                            + c["height_m"] * c["eye_corner_y"]
                            - 0.0015 * (1 - t * t)
                        )
                        self.assertAlmostEqual(
                            y + closure_delta(x, y, "upper", c), seam
                        )
                    guide = upper_change(t, self.style["calibration"], validate(p))
                    expected = (
                        y
                        - (
                            self.style["calibration"]["origin_y_m"]
                            + c["height_m"]
                            * (
                                self.style["calibration"]["eye_corner_y"]
                                + self.style["calibration"]["eye_upper_arch"]
                                * upper_shape(t, self.style["calibration"])
                            )
                        )
                    ) / c["height_m"]
                    self.assertAlmostEqual(guide, expected)

    def test_interval_contains_shifted_peak_and_identity_extension(self):
        sys.path.insert(0, str(Path(__file__).parents[1] / "facial_surface"))
        try:
            from interval_bounds import I, upper_coordinate_interval
        finally:
            sys.path.pop(0)
        for a in (-0.1, 0.1):
            c = dict(self.base, eye_upper_asymmetry=a)
            for lo, hi in [(-1.2, -0.9), (-0.2, 0.2), (0.4, 0.6), (0.9, 1.2)]:
                result = upper_coordinate_interval(I(lo, hi), c)
                for i in range(101):
                    v = eye_warp(lo + (hi - lo) * i / 100, c)
                    self.assertLessEqual(result.lo, v)
                    self.assertGreaterEqual(result.hi, v)

    def test_actual_outer_inverse_round_trip_and_explicit_clamp_domain(self):
        sources = {
            n: (RECIPE / n).read_text()
            for n in ("face.py", "eye_socket.py", "lateral_fit.py")
        }
        source = adapt_upper_sources(sources, True)["lateral_fit.py"]
        node = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "eye_rim_x_at_height"
        )
        env = dict(upper_shape=upper_shape)
        exec(
            compile(
                ast.Module(body=[node], type_ignores=[]), "<actual inverse>", "exec"
            ),
            env,
        )
        for a in (-0.1, 0.1):
            c = dict(self.base, eye_upper_asymmetry=a)
            contour = self.contour(c, True)
            inverse = env["eye_rim_x_at_height"]
            for t in (0.4, 0.45, 0.7, 0.9, 0.999):
                x, y = contour(1, t, True)
                self.assertAlmostEqual(inverse(y, c), x, places=12)
            # The inherited function saturates above its branch, rather than
            # claiming to invert the whole upper lid through its shifted peak.
            _, ceiling = contour(1, 0.4, True)
            self.assertAlmostEqual(
                inverse(ceiling + 0.01, c), contour(1, 0.4, True)[0], places=12
            )

    def test_native_depth_is_enclosed_by_matching_interval_field(self):
        sys.path.insert(0, str(Path(__file__).parents[1] / "facial_surface"))
        try:
            from interval_bounds import I, orbital_weight
        finally:
            sys.path.pop(0)
        source = adapt_face_source((RECIPE / "face.py").read_text(), True)
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name in HELPER_NAMES | {"depth"}
        ]
        for a in (-0.1, 0.1):
            c = dict(self.base, eye_upper_asymmetry=a)
            env = dict(
                C=c,
                H=c["height_m"],
                Y0=c["origin_y_m"],
                sqrt=math.sqrt,
                exp=math.exp,
                base_depth=lambda x, y: 0,
                rim_depth=lambda x, y: 1,
            )
            exec(
                compile(
                    ast.Module(body=nodes, type_ignores=[]), "<actual depth>", "exec"
                ),
                env,
            )
            for t in (-1.1, -0.8, -0.1, 0, 0.4, 0.9, 1.1):
                x = c["height_m"] * (c["eye_center_x"] + c["eye_half_width"] * t)
                for rel in (-0.05, -0.001, 0, 0.001, 0.05):
                    h = c["eye_corner_y"] + rel
                    actual = env["depth"](x, c["origin_y_m"] + c["height_m"] * h)
                    bound = orbital_weight(
                        I(x - 1e-12, x + 1e-12), I(h - 1e-12, h + 1e-12), c
                    )
                    self.assertLessEqual(bound.lo, actual)
                    self.assertGreaterEqual(bound.hi, actual)

    def test_generated_request_embeds_matching_helpers_and_retains_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture input; never executed")
            for amount in (0, 0.05):
                p = dict(
                    self.p,
                    upper_asymmetry=amount,
                    supplemental_lashes=0,
                    lid_fold=0,
                    lash_contour_follow=1,
                )
                request = build_request(
                    p, "assets/aster/general_face_eyes_cpu_warp/", root, blink=True
                )
                tree = ast.parse(request["code"])
                compiled = {
                    node.args[1].value: node.args[0].value
                    for node in ast.walk(tree)
                    if isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Name)
                    and node.func.id == "compile"
                    and len(node.args) > 1
                    and isinstance(node.args[0], ast.Constant)
                    and isinstance(node.args[1], ast.Constant)
                }
                for name in ("face", "semantic_eye_runtime"):
                    self.assertEqual("def eye_warp(" in compiled[name], bool(amount))
                    self.assertNotIn("from contour_warp import", compiled[name])
                self.assertIn("RETAINED_UPPER_SHAPE_SOURCES", request["code"])
                self.assertLessEqual(len(request["outputs"]), 64)

    def test_native_derived_sources_share_contract(self):
        sources = {
            n: (RECIPE / n).read_text()
            for n in ("face.py", "eye_socket.py", "lateral_fit.py")
        }
        self.assertIs(adapt_upper_sources(sources, False), sources)
        altered = adapt_upper_sources(sources, True)
        self.assertIn("q = eye_warp(t, C) if y >= cy else t", altered["face.py"])
        self.assertIn("abs(eye_warp(t, C) if y >= cy else t)", altered["eye_socket.py"])
        self.assertIn("shape=upper_shape(t,c) if upper else", altered["lateral_fit.py"])
        for source in altered.values():
            compile(source, "<derived native>", "exec")


if __name__ == "__main__":
    unittest.main()
