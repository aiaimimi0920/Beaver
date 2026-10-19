import ast
import copy
import json
import math
from pathlib import Path
import unittest
from source_mouth_templates import (
    apply_mouth_template,
    split_templates,
    adapt_mouth_face,
    adapt_mouth_companions,
)
from source_family_templates import digest

HERE = Path(__file__).parent
RECIPE = HERE.parent / "checkpoints/current"


class MouthTemplateTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.template = json.loads(
            (HERE.parent / "face_components/family-templates.json").read_text()
        )["bowed-mouth-contour-v1"]

    def functions(self):
        style, report = apply_mouth_template(self.style, [self.template])
        c = style["calibration"]
        source = adapt_mouth_face((RECIPE / "face.py").read_text(), True)
        names = {"mouth_profile_shift", "mouth_rest_seam_y"}
        ns = dict(C=c, H=c["height_m"], Y0=c["origin_y_m"])
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name in names
        ]
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "mouth", "exec"), ns)
        return ns, report

    def test_default_identity(self):
        s, r = apply_mouth_template(self.style, [])
        self.assertEqual(s, self.style)
        self.assertIsNone(r)
        for name in ("mouth.py", "textures.py", "crease_rules.py"):
            src = (RECIPE / name).read_text()
            self.assertEqual(src, adapt_mouth_companions(name, src))
        src = (RECIPE / "face.py").read_text()
        self.assertEqual(src, adapt_mouth_face(src))

    def test_contour_symmetry_endpoints_and_slope_bound(self):
        ns, r = self.functions()
        f = ns["mouth_profile_shift"]
        c = ns["C"]
        radius = 0.7 * ns["H"] * c["mouth_half_width"]
        eps = 1e-8
        for i in range(1001):
            x = radius * i / 1000
            self.assertAlmostEqual(f(x), f(-x), places=15)
            self.assertLessEqual(f(x), r["seam_height_change_bound_m"] + 1e-15)
            self.assertLessEqual(
                abs((f(x + eps) - f(x - eps)) / (2 * eps)),
                r["seam_slope_change_bound"] + 1e-7,
            )
        self.assertEqual(f(0), 0)
        self.assertEqual(f(radius), 0)
        self.assertEqual(f(radius * 2), 0)
        self.assertAlmostEqual(
            (f(radius + eps) - f(radius - eps)) / (2 * eps), 0, places=5
        )
        self.assertGreater(f(radius / math.sqrt(3)), 0.0009)

    def test_shared_contour_and_positive_gap(self):
        ns, _ = self.functions()
        c = ns["C"]
        ns.update(
            cos=math.cos,
            sin=math.sin,
            MOUTH_Y=ns["Y0"] + ns["H"] * c["mouth_height"],
            REST_GAP=ns["H"] * c["mouth_rest_half_gap"],
        )
        src = adapt_mouth_companions(
            "mouth.py", (RECIPE / "mouth.py").read_text(), True
        )
        nodes = [
            n
            for n in ast.parse(src).body
            if isinstance(n, ast.FunctionDef)
            and n.name in {"rest_seam_y", "mouth_half_contour"}
        ]
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "contour", "exec"), ns)
        for i in range(1, 100):
            a = math.pi * i / 100
            x, upper = ns["mouth_half_contour"](a)
            _, lower = ns["mouth_half_contour"](-a)
            self.assertGreater(upper, lower)
            self.assertAlmostEqual((upper + lower) / 2, ns["mouth_rest_seam_y"](x))
        for name in ("textures.py", "crease_rules.py"):
            self.assertIn(
                "mouth_rest_seam_y(x)",
                adapt_mouth_companions(name, (RECIPE / name).read_text(), True),
            )

    def test_combined_lip_interval_at_shoulder_boundary(self):
        from source_lips import apply_lip_relief, adapt_lip_source
        from interval_bounds import I, raw_without_origin
        from study import scope, fit_profile

        style, _ = apply_mouth_template(self.style, [self.template])
        style, _ = apply_lip_relief(
            style, {"upper_lip_bow_height": 0.003, "lower_lip_relief_scale": 1.25}
        )
        c = style["calibration"]
        source = adapt_mouth_face(
            adapt_lip_source((RECIPE / "face.py").read_text(), c), True
        )
        env = scope(source, c, fit_profile(source, c), 0)
        radius = 0.7 * c["height_m"] * c["mouth_half_width"]
        for center in (
            -radius,
            -radius / math.sqrt(3),
            0,
            radius / math.sqrt(3),
            radius,
        ):
            for h in (0.11, 0.15, 0.17, 0.20):
                bound = raw_without_origin(
                    I(center - 0.0001, center + 0.0001), I(h - 0.0001, h + 0.0001), c
                )
                for dx in (-0.0001, 0, 0.0001):
                    for dh in (-0.0001, 0, 0.0001):
                        value = (
                            env["base_depth"](
                                center + dx, c["origin_y_m"] + c["height_m"] * (h + dh)
                            )
                            - env["Z0"]
                        )
                        self.assertLessEqual(bound.lo - 1e-9, value)
                        self.assertGreaterEqual(bound.hi + 1e-9, value)

    def test_pigment_peak_follows_shared_seam(self):
        ns, _ = self.functions()
        ns.update(
            exp=math.exp,
            STYLE={"lip_pigment_strength": 0.2, "lip_pigment_width_m": 0.001},
        )
        source = adapt_mouth_companions(
            "textures.py", (RECIPE / "textures.py").read_text(), True
        )
        node = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "lip_pigment"
        )
        exec(compile(ast.Module(body=[node], type_ignores=[]), "pigment", "exec"), ns)
        for t in (-0.8, -0.4, 0, 0.4, 0.8):
            x = t * ns["H"] * ns["C"]["mouth_half_width"]
            y = ns["mouth_rest_seam_y"](x)
            peak = ns["lip_pigment"](x, y)
            self.assertAlmostEqual(peak, 0.2 * (1 - t * t) ** 0.35)
            self.assertGreater(peak, ns["lip_pigment"](x, y + 0.001))

    def test_template_ownership_and_rejection(self):
        before = copy.deepcopy(self.style)
        s, _ = apply_mouth_template(self.style, [self.template])
        self.assertEqual(self.style, before)
        self.assertEqual(
            set(s["calibration"]) - set(before["calibration"]),
            {"mouth_seam_bow_height", "mouth_seam_shoulder_radius_ratio"},
        )
        for a in (True, float("inf"), -0.1, 0.005):
            bad = copy.deepcopy(self.template)
            bad["seam_bow_height"] = a
            if math.isfinite(a):
                bad["template_sha256"] = digest(
                    {k: v for k, v in bad.items() if k != "template_sha256"}
                )
            with self.assertRaises(ValueError):
                apply_mouth_template(self.style, [bad])
        with self.assertRaises(ValueError):
            split_templates([self.template, self.template])
        face = {"slot": "face"}
        self.assertEqual(
            split_templates([face, self.template]), ([face], [self.template])
        )


if __name__ == "__main__":
    unittest.main()
