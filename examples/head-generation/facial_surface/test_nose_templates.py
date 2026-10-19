import ast
import copy
import json
import math
from pathlib import Path
import unittest
from source_nose_templates import (
    apply_nose_template,
    adapt_nose_face,
    adapt_nose_companion,
)
from source_family_templates import digest

HERE = Path(__file__).parent
RECIPE = HERE.parent / "checkpoints/current"


class NoseTemplateTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.t = json.loads(
            (HERE.parent / "face_components/family-templates.json").read_text()
        )["short-rounded-nose-v1"]

    def test_identity_and_owned_fields(self):
        result, r = apply_nose_template(self.style, [])
        self.assertEqual(result, self.style)
        self.assertIsNone(r)
        original = copy.deepcopy(self.style)
        new, r = apply_nose_template(self.style, [self.t])
        self.assertEqual(self.style, original)
        self.assertEqual(
            {
                k
                for k in new["calibration"]
                if new["calibration"][k] != original["calibration"].get(k)
            },
            {"nose_stations", "nose_template_mode"},
        )
        for name in ("nose_refine.py", "nose_fair.py"):
            source = (RECIPE / name).read_text()
            self.assertEqual(source, adapt_nose_companion(name, source))
        source = (RECIPE / "face.py").read_text()
        self.assertEqual(source, adapt_nose_face(source))

    def test_endpoints_center_and_edges_are_smooth(self):
        style, _ = apply_nose_template(self.style, [self.t])
        c = style["calibration"]
        source = adapt_nose_face((RECIPE / "face.py").read_text(), True)
        ns = {}
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef)
            and n.name in ("nasal_projection_curve", "nose_profile_factor")
        ]
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "nose", "exec"), ns)
        f = ns["nasal_projection_curve"]
        p = c["nose_stations"]
        eps = 1e-8
        for h in (0.19, 0.4):
            self.assertEqual(f(h, 1, p), 0)
            self.assertAlmostEqual(
                (f(h + eps, 1, p) - f(h - eps, 1, p)) / (2 * eps), 0, places=5
            )
        factor = ns["nose_profile_factor"]
        self.assertEqual(factor(0, 0.3), 1)
        self.assertEqual(factor(1, 0.3), 0)
        self.assertAlmostEqual((factor(eps, 0.3) - factor(0, 0.3)) / eps, 0, places=5)
        self.assertAlmostEqual(
            (factor(1, 0.3) - factor(1 - eps, 0.3)) / eps, 0, places=5
        )
        for i in range(1001):
            h = 0.19 + 0.21 * i / 1000
            self.assertGreaterEqual(f(h, 1, p), -1e-12)
            self.assertLessEqual(f(h, 1, p), max(r[1] for r in p) + 1e-12)

    def test_interval_covers_projection_caps_and_cross_section(self):
        from interval_bounds import I, raw_without_origin
        from study import scope, fit_profile

        style, _ = apply_nose_template(self.style, [self.t])
        c = style["calibration"]
        source = adapt_nose_face((RECIPE / "face.py").read_text(), True)
        env = scope(source, c, fit_profile(source, c), 0)
        for h in (0.19, 0.22, 0.25, 0.286, 0.303, 0.34, 0.4):
            for x in (0, 0.004, 0.01, 0.017):
                bound = raw_without_origin(
                    I(x - 0.00001, x + 0.00001), I(h - 0.00001, h + 0.00001), c
                )
                for dx in (-0.00001, 0, 0.00001):
                    for dh in (-0.00001, 0, 0.00001):
                        z = (
                            env["base_depth"](
                                x + dx, c["origin_y_m"] + c["height_m"] * (h + dh)
                            )
                            - env["Z0"]
                        )
                        self.assertLessEqual(bound.lo - 1e-9, z)
                        self.assertGreaterEqual(bound.hi + 1e-9, z)

    def test_curve_source_contract_rejects_changed_endpoint_anchor(self):
        source = (
            (RECIPE / "face.py").read_text().replace("[slopes[0]]", "[slopes[0]+0.1]")
        )
        with self.assertRaisesRegex(ValueError, "source contract"):
            adapt_nose_face(source, True)

    def test_native_caps_and_invalid_templates(self):
        refine = adapt_nose_companion(
            "nose_refine.py", (RECIPE / "nose_refine.py").read_text(), True
        )
        fair = adapt_nose_companion(
            "nose_fair.py", (RECIPE / "nose_fair.py").read_text(), True
        )
        for guard in ("maximum<.003", "poly.normal.dot(n)>.8"):
            self.assertIn(guard, refine)
        for guard in (
            "maximum<.0008",
            "p.normal.dot(n)>.85",
            "Nose arc reverses direction",
        ):
            self.assertIn(guard, fair)
        for kind in ("width", "height", "projection"):
            t = copy.deepcopy(self.t)
            t["nose_stations"][3][
                {"height": 0, "projection": 1, "width": 2}[kind]
            ] += 0.02
            t["template_sha256"] = digest(
                {k: v for k, v in t.items() if k != "template_sha256"}
            )
            with self.assertRaises(ValueError):
                apply_nose_template(self.style, [t])


if __name__ == "__main__":
    unittest.main()
