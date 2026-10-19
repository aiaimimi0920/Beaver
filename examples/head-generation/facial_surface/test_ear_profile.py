import ast
import json
import math
from pathlib import Path
import unittest
from source_components import apply_components
from source_ear_profile import adapt_ear_profile


class EarProfileTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        self.source = (root / "ears.py").read_text()
        self.style = json.loads((root / "face_style.json").read_text())
        self.parameters = {
            "ear_tip_length_scale": 1.7,
            "ear_point_strength": 1,
            "ear_tip_lift": 0.025,
        }

    def functions(self, config):
        source = adapt_ear_profile(self.source, config)
        nodes = [
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef)
            and n.name
            in ("membrane_offset", "pointed_ear_envelope", "ear_distal_weight")
        ]

        def curve(u, axis, points):
            for a, b in zip(points, points[1:]):
                if u <= b[0]:
                    return a[axis] + (b[axis] - a[axis]) * (u - a[0]) / (b[0] - a[0])
            return points[-1][axis]

        ns = {"sin": math.sin, "pi": math.pi, "curve": curve}
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "ear-test", "exec"), ns)
        return ns

    def test_identity_exact_source_and_calibration(self):
        result, report = apply_components(self.style, {})
        self.assertEqual(result, self.style)
        self.assertIsNone(report)
        self.assertEqual(
            adapt_ear_profile(self.source, result["calibration"]), self.source
        )

    def test_ear_only_keys_and_pinned_root_offsets(self):
        style, report = apply_components(self.style, self.parameters)
        c = style["calibration"]
        self.assertEqual(set(report["after"]), set(self.parameters))
        for key in self.parameters:
            c.pop(key)
        self.assertEqual(style, self.style)
        style, _ = apply_components(self.style, self.parameters)
        c = style["calibration"]
        f = self.functions(c)["membrane_offset"]
        for i in range(21):
            self.assertEqual(f(i / 20, 0, c, 0.3), (0.0, 0.0))
        self.assertIn("u * u", adapt_ear_profile(self.source, c))

    def test_point_envelope_and_larger_distal_span(self):
        style, _ = apply_components(self.style, self.parameters)
        c = style["calibration"]
        ns = self.functions(c)
        self.assertEqual(ns["pointed_ear_envelope"](0), 0)
        self.assertEqual(ns["pointed_ear_envelope"](1), 0)
        self.assertEqual(ns["pointed_ear_envelope"](0.7), 1)
        original = self.functions(self.style["calibration"])["membrane_offset"]
        self.assertGreater(
            ns["membrane_offset"](0.7, 1, c, 0.3)[0],
            original(0.7, 1, self.style["calibration"], 0.3)[0],
        )
        for t in (0.1, 0.3, 0.7, 0.9):
            self.assertEqual(
                ns["membrane_offset"](t, 1, c, 0.3)[1],
                original(t, 1, self.style["calibration"], 0.3)[1],
            )

    def test_invalid_controls_and_contract_fail(self):
        for key in self.parameters:
            for value in (True, float("nan"), float("inf"), -1, 3):
                with self.assertRaises(ValueError):
                    apply_components(self.style, {key: value})
        with self.assertRaises(ValueError):
            adapt_ear_profile("bad", self.parameters)

    def test_single_control_populates_complete_native_calibration(self):
        for key, value in self.parameters.items():
            style, _ = apply_components(self.style, {key: value})
            self.assertTrue(set(self.parameters).issubset(style["calibration"]))
            self.functions(style["calibration"])["membrane_offset"](
                0.7, 1, style["calibration"], 0.3
            )

    def test_standalone_partial_active_calibration_uses_defaults(self):
        config = dict(self.style["calibration"], ear_tip_length_scale=1.7)
        self.functions(config)["membrane_offset"](0.7, 1, config, 0.3)

    def test_proximal_and_lower_attachment_neighborhood_is_unchanged(self):
        style, _ = apply_components(self.style, self.parameters)
        c = style["calibration"]
        ns = self.functions(c)
        old = self.functions(self.style["calibration"])["membrane_offset"]
        for t in (0, 0.1, 0.25, 0.5, 0.7, 0.9, 1):
            for u in (0, 0.22, 0.35, 0.5, 1):
                if t <= 0.25 or u <= 0.35:
                    self.assertEqual(ns["ear_distal_weight"](t, u), 0)
                    self.assertEqual(
                        ns["membrane_offset"](t, u, c, 0.3),
                        old(t, u, self.style["calibration"], 0.3),
                    )
