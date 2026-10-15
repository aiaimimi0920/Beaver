import copy
import math
import unittest
from controls import (PRESETS, axes, validate, transform_point, inverse_point,
                      mouth_delta, inverse_transpose_normal, jacobian, measure_profile,
                      pivot_from_style)
from prompt_contract import select_intent
from bundle import build_request, RECIPE


class ControlTests(unittest.TestCase):
    pivot = (0, 0, 1.641)

    def test_identity_exact_and_unmutated(self):
        values = [(0.0, -0.0, 1.641), (1e-30, -123.456, 1.7)]
        original = copy.deepcopy(PRESETS)
        for p in values:
            self.assertEqual(transform_point(p, {}, self.pivot), p)
            self.assertEqual(inverse_point(p, {}, self.pivot), p)
            self.assertEqual(mouth_delta(p, {}), p)
        self.assertEqual(inverse_transpose_normal((.2, .3, .4), {}), (.2, .3, .4))
        result = select_intent({"preset": "柔和宽脸"})
        result["proportions"]["width"] = 1
        self.assertEqual(PRESETS, original)

    def test_inverse_mirror_slope_and_jacobian(self):
        for config in PRESETS.values():
            self.assertGreater(jacobian(config), 0)
            p = (.074, -.081, 1.66)
            q = transform_point(p, config, self.pivot)
            recovered = inverse_point(q, config, self.pivot)
            for a, b in zip(p, recovered):
                self.assertAlmostEqual(a, b)
            mirrored = transform_point((-p[0], p[1], p[2]), config, self.pivot)
            self.assertEqual(mirrored, (-q[0], q[1], q[2]))
            for axis, scale in enumerate(axes(config)):
                offset = list(p)
                offset[axis] += .001
                shifted = transform_point(offset, config, self.pivot)
                self.assertAlmostEqual((shifted[axis] - q[axis]) / .001, scale)

    def test_deltas_and_normals(self):
        config = PRESETS["slender"]
        p, d = (.02, -.08, 1.63), (.001, -.002, -.003)
        q = transform_point(p, config, self.pivot)
        r = transform_point(tuple(a + b for a, b in zip(p, d)), config, self.pivot)
        for a, b in zip(mouth_delta(d, config), (b - a for a, b in zip(q, r))):
            self.assertAlmostEqual(a, b)
        tangent, normal = (2, 3, 1), (3, -2, 0)
        t = mouth_delta(tangent, config)
        n = inverse_transpose_normal(normal, config)
        self.assertAlmostEqual(sum(a * b for a, b in zip(t, n)), 0)
        self.assertAlmostEqual(sum(v * v for v in n), 1)

    def test_measured_profiles_differ(self):
        points = [(-.08, -.1, 1.55), (.08, .02, 1.77)]
        base = measure_profile(points)
        measured = {}
        for name, config in PRESETS.items():
            measured[name] = measure_profile(transform_point(p, config, self.pivot) for p in points)
            for key, span in base.items():
                self.assertAlmostEqual(measured[name][key] / span, config[key])
        self.assertGreater(measured["soft_wide"]["width"], measured["slender"]["width"])
        self.assertLess(measured["soft_wide"]["height"], measured["slender"]["height"])

    def test_rejected_configuration(self):
        for value in [True, False, float("nan"), float("inf"), "1", None, .91, 1.09]:
            with self.assertRaises(ValueError):
                validate({"width": value})
        for config in [{"age": 2}, [], None, {"height": .939}, {"depth": 1.041}]:
            with self.assertRaises(ValueError):
                validate(config)
        for intent in [{}, {"prompt": "年轻"}, {"preset": "unknown"},
                       {"preset": "baseline", "proportions": {}}, {"preset": []}]:
            with self.assertRaises(ValueError):
                select_intent(intent)

    def test_bundle_fresh_inputs_and_no_upstream_changes(self):
        before = {p.name: p.read_bytes() for p in RECIPE.glob("*.py")}
        request = build_request({"preset": "soft_wide"}, "assets/aster/general_face_test/")
        compile(request["code"], "job", "exec")
        self.assertIn('head_recovery_49', request["code"])
        self.assertIn('head_recovery_617', request["code"])
        self.assertIn('apply_general_face(', request["code"])
        paths = [entry["path"] for entry in request["outputs"]]
        self.assertEqual(len(paths), len(set(paths)))
        self.assertTrue(all(p.startswith("assets/aster/general_face_test/") for p in paths))
        self.assertEqual(before, {p.name: p.read_bytes() for p in RECIPE.glob("*.py")})
        with self.assertRaises(ValueError):
            build_request({"preset": "baseline"}, "assets/aster/head_recovery_664/")


if __name__ == "__main__":
    unittest.main()
