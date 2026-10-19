"""CPU checks for optional preset-specific initial side-lash guides."""
import ast
import json
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate
from source_upper_warp import adapt_face_source, adapt_upper_sources, HELPER_NAMES
from source_lateral_rim import adapt_lateral_rim, adapt_lateral_depth


class LateralPresetTests(unittest.TestCase):
    def test_disabled_is_identical_and_range_is_discrete(self):
        source = (RECIPE / "eye_accents.py").read_text()
        self.assertEqual(source, adapt_lateral_rim(source, False))
        self.assertIn("eye_rim_x_at_height", adapt_lateral_rim(source, True))
        for value in (-1, .5, 2, True):
            with self.assertRaises(ValueError):
                validate({"lateral_aperture_follow": value})
        with self.assertRaises(ValueError):
            adapt_lateral_rim("unexpected source", True)

    def test_depth_mode_retains_safety_checks(self):
        source = (RECIPE / "lateral_fit.py").read_text()
        self.assertEqual(source, adapt_lateral_depth(source, False))
        changed = adapt_lateral_depth(source, True)
        self.assertIn("len(stations)==len(centers)", changed)
        self.assertIn('"initial_chord_shaping_applied":False', changed)
        for guard in ("distance < .012", "< .004", "paired_shifts", "support_moves"):
            self.assertIn(guard, changed)
        node = next(n for n in ast.parse(changed).body if
                    isinstance(n, ast.FunctionDef) and n.name == "seating_delta")
        scope = {}
        exec(compile(ast.Module(body=[node], type_ignores=[]), "<seat>", "exec"), scope)
        with self.assertRaises(AssertionError):
            scope["seating_delta"]((0,0,0),(.006,0,0),(0,1,0),1,.00035)
        base = json.loads((RECIPE / "face_style.json").read_text())
        self.assertEqual(adapt_style(base, {"lateral_aperture_follow":0}),base)
        self.assertEqual(adapt_style(base, {"lateral_aperture_follow":1})[
            "accent_lateral_depth_blend"],0)

    def test_no_lost_neutral_safety_guard(self):
        source = adapt_lateral_depth((RECIPE / "lateral_fit.py").read_text(), True)
        for message in ("Missing own-face lateral support", "Lateral seating requires structural redesign",
                        "Companion/parent guide mismatch exceeds local correction",
                        "Eye rim attachment needs structural redesign"):
            self.assertIn(message, source)
        accents = adapt_lateral_rim((RECIPE / "eye_accents.py").read_text(), True)
        self.assertIn("target_x = anchor_x+side*H*offset", accents)
        self.assertIn("value[2]", accents)
        self.assertIn("phase = max(0.0,min(1.0,f/.20))", accents)

    def test_lateral_blink_is_independent_and_bounded(self):
        from blink_runtime import closure_delta
        base = json.loads((RECIPE / "face_style.json").read_text())
        old = adapt_style(base, {})["calibration"]
        new = adapt_style(base, {"lateral_blink_follow":.70})["calibration"]
        x = old["height_m"]*old["eye_center_x"]
        y = old["origin_y_m"]+old["height_m"]*old["eye_corner_y"]
        for kind in ("skin", "upper", "lower"):
            self.assertEqual(closure_delta(x,y,kind,old),closure_delta(x,y,kind,new))
        self.assertAlmostEqual(closure_delta(x,y,"lateral",new),
                               .70/.85*closure_delta(x,y,"lateral",old))
        for invalid in (.649,.851,True,float("nan")):
            with self.assertRaises(ValueError):
                validate({"lateral_blink_follow":invalid})

    def test_real_inverse_guide_endpoints_mirror_and_offset(self):
        original = json.loads((RECIPE / "face_style.json").read_text())
        style = adapt_style(original, dict(corner_slope_offset=-.014,
            upper_arch=.95, lower_arch=.55, upper_asymmetry=.12,
            lower_shoulder=.8, lower_horizontal_roundness=1.10,
            lateral_aperture_follow=1))
        c = style["calibration"]
        scope = dict(C=c, H=c["height_m"], Y0=c["origin_y_m"], STYLE=style)
        sources = adapt_upper_sources({n:(RECIPE / n).read_text() for n in
            ("face.py", "eye_socket.py", "lateral_fit.py")}, True)
        selected = HELPER_NAMES | {"eye_contour", "eye_rim_x_at_height"}
        for source in (sources["face.py"], sources["lateral_fit.py"]):
            nodes = [n for n in ast.parse(source).body if
                     isinstance(n, ast.FunctionDef) and n.name in selected]
            exec(compile(ast.Module(body=nodes, type_ignores=[]), "<guide>", "exec"), scope)
        source = adapt_lateral_rim((RECIPE / "eye_accents.py").read_text(), True)
        node = next(n for n in ast.parse(source).body if
                    isinstance(n, ast.FunctionDef) and n.name == "lateral_position")
        exec(compile(ast.Module(body=[node], type_ignores=[]), "<position>", "exec"), scope)
        position, contour = scope["lateral_position"], scope["eye_contour"]
        last = float("inf")
        for i in range(101):
            f=i/100
            x,y=position(1,f,0)
            self.assertLessEqual(x,last+1e-10)
            last=x
            self.assertEqual(position(-1,f,0),(-x,y))
            self.assertAlmostEqual(position(1,f,.002)[0]-x,c["height_m"]*.002)
        self.assertAlmostEqual(position(1,0,0)[0],contour(1,1,False)[0])
        self.assertAlmostEqual(position(1,1,0)[0],contour(1,.5,False)[0])
        self.assertAlmostEqual(position(1,0,0)[1],contour(1,.95,True)[1])
        self.assertAlmostEqual(position(1,1,0)[1],contour(1,.5,False)[1])


if __name__ == "__main__":
    unittest.main()
