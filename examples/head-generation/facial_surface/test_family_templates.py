import copy
import json
from pathlib import Path
import unittest
from source_family_templates import (
    apply_family_templates,
    digest,
    instrument_ear_constraints,
)
from source_silhouette import apply_silhouette, adapt_ear_envelope

HERE = Path(__file__).parent
RECIPE = HERE.parent / "checkpoints/current"


class TemplateTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.template = json.loads(
            (HERE.parent / "face_components/family-templates.json").read_text()
        )["soft-oval-width-v1"]

    def signed(self, change):
        t = copy.deepcopy(self.template)
        change(t)
        t["template_sha256"] = digest(
            {k: v for k, v in t.items() if k != "template_sha256"}
        )
        return t

    def test_v2_contract_and_source_guards(self):
        from source_family_templates import transport_ear_constraints

        t = json.loads(
            (HERE.parent / "face_components/family-templates.json").read_text()
        )["soft-oval-width-v2"]
        result, report = apply_family_templates(self.style, [t])
        self.assertEqual(report["template"]["version"], 2)
        self.assertEqual(
            result["calibration"]["width_stations"], self.template["width_stations"]
        )
        source = (RECIPE / "ear_boundary.py").read_text()
        adapted = transport_ear_constraints(
            source, self.style["calibration"]["width_stations"]
        )
        for guard in (
            "limit=.003",
            "limit=.004",
            "poly.normal.dot(n)<=0",
            "coherent_bridge_targets",
        ):
            self.assertIn(guard, adapted)
        with self.assertRaises(ValueError):
            transport_ear_constraints(adapted, [])

    def test_protected_root_template_and_no_residual(self):
        from source_silhouette import bezier_segments

        t = json.loads(
            (HERE.parent / "face_components/family-templates.json").read_text()
        )["root-protected-round-chin-v1"]
        result, r = apply_family_templates(self.style, [t])
        self.assertEqual(r["protected_height_range"], [0.16, 1.0])
        bad = copy.deepcopy(t)
        bad["width_stations"][3][1] += 0.001
        bad["template_sha256"] = digest(
            {k: v for k, v in bad.items() if k != "template_sha256"}
        )
        with self.assertRaisesRegex(ValueError, "Protected"):
            apply_family_templates(self.style, [bad])
        self.assertEqual(
            bezier_segments(self.style["calibration"]["width_stations"])[4:],
            bezier_segments(result["calibration"]["width_stations"])[4:],
        )
        with self.assertRaisesRegex(ValueError, "not supported"):
            apply_family_templates(self.style, [t], {"chin_width": -0.01})

    def test_legacy_identity(self):
        for templates in (None, []):
            result, report = apply_family_templates(self.style, templates)
            self.assertEqual(result, self.style)
            self.assertIsNone(report)

    def test_exact_owned_calibration(self):
        original = copy.deepcopy(self.style)
        result, report = apply_family_templates(self.style, [self.template])
        a = self.style["calibration"]
        b = result["calibration"]
        self.assertEqual({k for k in a if a[k] != b[k]}, {"width_stations"})
        self.assertEqual(self.style, original)
        self.assertLessEqual(report["template_half_width_change_bound_m"], 0.012)
        self.assertGreater(report["template_half_width_change_bound_m"], 0.002)
        self.assertEqual(report["template"], self.template)

    def test_residual_budget_stays_separate(self):
        result, _ = apply_family_templates(self.style, [self.template])
        final, r = apply_silhouette(result, {"chin_width": -0.04})
        self.assertEqual(r["original_width_stations"], self.template["width_stations"])
        self.assertLessEqual(r["continuous_half_width_change_bound_m"], 0.002)
        with self.assertRaises(ValueError):
            apply_silhouette(result, {"jaw_width": 1})

    def test_hash_or_base_mismatch(self):
        bad = copy.deepcopy(self.template)
        bad["width_stations"][0][1] += 0.001
        with self.assertRaises(ValueError):
            apply_family_templates(self.style, [bad])
        self.style["calibration"]["width_stations"][0][1] += 0.001
        with self.assertRaises(ValueError):
            apply_family_templates(self.style, [self.template])

    def test_protected_tangent_not_only_station(self):
        t = self.signed(lambda t: t["width_stations"][6].__setitem__(1, 0.33))
        with self.assertRaisesRegex(ValueError, "Protected"):
            apply_family_templates(self.style, [t])

    def test_absolute_domain_and_limit(self):
        for change in (
            lambda t: t["width_stations"][0].__setitem__(1, 0.8),
            lambda t: t.__setitem__("max_baseline_width_change_m", 0.1),
            lambda t: t.__setitem__("version", True),
            lambda t: t.__setitem__("slot", "mouth"),
        ):
            with self.assertRaises(ValueError):
                apply_family_templates(self.style, [self.signed(change)])

    def test_explicit_residual_compatibility(self):
        apply_family_templates(self.style, [self.template], {"chin_width": -0.04})
        for residual in (
            {"jaw_width": 0.02},
            {"chin_width": 0.04},
            {"chin_softness": 1},
        ):
            with self.assertRaisesRegex(ValueError, "not supported"):
                apply_family_templates(self.style, [self.template], residual)

    def test_overlap_and_unknown_data_rejected(self):
        for templates in ({}, [self.template, self.template]):
            with self.assertRaises(ValueError):
                apply_family_templates(self.style, templates)

    def test_attachment_diagnostics_and_full_base(self):
        source = (RECIPE / "ear_boundary.py").read_text()
        adapted = instrument_ear_constraints(source)
        self.assertIn("template_envelope_proposed_limited_max_m", adapted)
        self.assertIn("'retained_shape_constraints_active':True", adapted)
        self.assertIn("poly.normal.dot(n)<=0", adapted)
        width = adapt_ear_envelope(
            (RECIPE / "ear_width.py").read_text(),
            self.style["calibration"]["width_stations"],
        )
        self.assertIn(repr(self.style["calibration"]["width_stations"]), width)
        self.assertIn("max(-.003,min(.003", width)


class AttachmentTransportTests(unittest.TestCase):
    def functions(self, adapted):
        import ast
        from source_family_templates import transport_ear_constraints

        source = (RECIPE / "ear_boundary.py").read_text()
        if adapted:
            source = transport_ear_constraints(source, [])
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef)
            and n.name in ("constrain_lower_envelope", "calibrate_opening_sweep")
        ]
        ns = {}
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "attachment", "exec"), ns)
        ns["family_width_delta"] = lambda z: 0
        return ns

    def test_zero_identity_and_translated_envelope(self):
        old = self.functions(False)
        new = self.functions(True)
        before = [(0, -0.1, 0), (0, 0.1, 1)]
        points = [(0.5, -0.2, 0.2), (-0.5, -0.2, 0.2)]
        fn = "constrain_lower_envelope"
        expected = old[fn](before, points, [0, 1])
        self.assertEqual(expected, new[fn](before, points, [0, 1]))
        new["family_width_delta"] = lambda z: 0.01
        shifted = [(x + (0.01 if x > 0 else -0.01), y, z) for x, y, z in points]
        result = new[fn](before, shifted, [0, 1])
        for i in range(2):
            self.assertAlmostEqual(
                result[i][0] - expected[i][0], 0.01 if i == 0 else -0.01
            )
            self.assertAlmostEqual(result[i][1], expected[i][1])
            self.assertLessEqual(
                sum((a - b) ** 2 for a, b in zip(result[i], shifted[i])) ** 0.5,
                0.003000001,
            )

    def test_opening_transport_preserves_depth_and_cap(self):
        import types, sys
        from unittest.mock import patch

        fake = types.ModuleType("ear_return")
        fake.return_weight = lambda v: 0
        fake.lower_return_weight = lambda v: 0
        old = self.functions(False)
        new = self.functions(True)
        before = [(0, -0.1, 0), (0, 0.1, 1)]
        points = [(0.5, -0.2, 0.2), (-0.5, -0.2, 0.2)]
        cfg = dict(
            origin_y_m=0,
            height_m=1,
            ear_post_return_width_m=0,
            ear_post_lower_return_width_m=0,
            ear_post_return_depth_m=0,
        )
        with patch.dict(sys.modules, {"ear_return": fake}):
            expected = old["calibrate_opening_sweep"](before, points, [0, 1], cfg)
            self.assertEqual(
                expected, new["calibrate_opening_sweep"](before, points, [0, 1], cfg)
            )
            new["family_width_delta"] = lambda z: 0.01
            shifted = [(x + (0.01 if x > 0 else -0.01), y, z) for x, y, z in points]
            result = new["calibrate_opening_sweep"](before, shifted, [0, 1], cfg)
        for i in range(2):
            self.assertAlmostEqual(
                result[i][0] - expected[i][0], 0.01 if i == 0 else -0.01
            )
            self.assertAlmostEqual(result[i][1], expected[i][1])
            self.assertLessEqual(
                sum((a - b) ** 2 for a, b in zip(result[i], shifted[i])) ** 0.5,
                0.004000001,
            )


if __name__ == "__main__":
    unittest.main()
