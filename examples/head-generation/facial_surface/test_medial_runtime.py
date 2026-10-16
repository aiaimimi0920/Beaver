import unittest
from runtime_field import base_delta


class MedialRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.c = dict(
            origin_y_m=0.0,
            height_m=1.0,
            width_stations="width",
            continuous_front_profile="front",
            continuous_back_profile="back",
            continuous_lateral_linear=0.28,
        )
        self.curve = lambda h, scale, key: dict(width=0.1, front=0.02, back=-0.1)[key]
        self.data = dict(
            parameters=dict(
                axial_smoothing=0.0, cheek_roundness=0.0, orbit_shallowing=0.0
            ),
            segments=[[0, 1, 0.02, 0, 0, 0]],
            transverse=dict(axial=[0] * 8, rounding=[0] * 8),
        )

    def test_zero_controls_remain_neutral(self):
        for h in (0.1, 0.15, 0.2, 0.5, 0.8, 0.85, 0.9):
            for x in (0.0, 0.001, 0.009, 0.019, 0.02, 0.04):
                self.assertEqual(base_delta(x, h, self.c, self.curve, self.data), 0.0)

    def test_medial_support_and_symmetry(self):
        self.data["parameters"]["axial_smoothing"] = 0.5
        f = lambda x: base_delta(x, 0.5, self.c, self.curve, self.data)
        self.assertGreater(f(0.005), 0.0)
        self.assertEqual(f(0.005), f(-0.005))
        self.assertAlmostEqual(f(0.02), 0.0, places=14)
        self.assertEqual(f(0.03), 0.0)
        e = 1e-6
        self.assertLess(abs(f(0.02 - e)) / e, 1e-7)

    def test_activation_preserves_existing_target(self):
        f = lambda: base_delta(0.005, 0.5, self.c, self.curve, self.data)
        self.data["parameters"]["axial_smoothing"] = 0.5
        a = f()
        self.data["parameters"]["axial_smoothing"] = 0.7
        self.assertEqual(f(), a)
        self.data["parameters"]["axial_smoothing"] = 0.25
        self.assertAlmostEqual(f(), a / 2)


class MedialNormalTests(unittest.TestCase):
    def test_zero_and_unrelated_regions_preserve_identity(self):
        from runtime_field import blend_medial_normal

        normal = object()
        c = {"origin_y_m": 0.0, "height_m": 1.0}
        data = {"parameters": {"axial_smoothing": 0.0}}
        self.assertIs(blend_medial_normal(normal, None, 1, 1, 0.6, c, data), normal)
        data["parameters"]["axial_smoothing"] = 0.5
        for region, side in ((19, 1), (1, 0)):
            self.assertIs(
                blend_medial_normal(normal, None, region, side, 0.6, c, data), normal
            )

    def test_active_weight_matches_reviewed_inline_formula(self):
        from runtime_field import blend_medial_normal

        class Normal:
            def lerp(self, other, weight):
                self.weight = weight
                return self

            def normalized(self):
                return self

        c = {"origin_y_m": 1.544, "height_m": 0.236}
        for amount in (0.1, 0.25, 0.5, 1.0):
            data = {"parameters": {"axial_smoothing": amount}}
            for height in (0.2, 0.4, 0.47, 0.55, 0.8):
                z = c["origin_y_m"] + height * c["height_m"] - 0.021
                actual_height = (z + 0.021 - c["origin_y_m"]) / c["height_m"]
                t = max(0.0, min(1.0, (actual_height - 0.4) / 0.15))
                expected = t * t * t * (10 + t * (-15 + 6 * t)) * min(1.0, 2 * amount)
                n = Normal()
                self.assertIs(blend_medial_normal(n, None, 1, 1, z, c, data), n)
                self.assertEqual(n.weight, expected)
