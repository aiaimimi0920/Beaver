import unittest
from pathlib import Path
from runtime_field import lower_medial_normal_weight
from source_normals import validate, adapt_pipeline


class LowerMedialNormalsTests(unittest.TestCase):
    def test_validation(self):
        self.assertEqual(validate(None), 0)
        self.assertEqual(validate({}), 0)
        for value in (False, -0.1, 1.1, float('nan'), float('inf'), '1'):
            with self.assertRaises(ValueError):
                validate({'lower_medial_blend': value})
        with self.assertRaises(ValueError):
            validate({'other': 1})
        self.assertEqual(validate({'lower_medial_blend': .5}), .5)

    def test_weight_window_and_taper(self):
        for h in (-1, 0, .05, .23, .35, .4, .8, 2):
            self.assertEqual(lower_medial_normal_weight(h, 1), 0)
        for h in (.1, .15, .2):
            self.assertAlmostEqual(lower_medial_normal_weight(h, 1), 1)
        for i in range(1001):
            h = i/1000
            w = lower_medial_normal_weight(h, 1)
            self.assertTrue(0 <= w <= 1)
            self.assertEqual(lower_medial_normal_weight(h, .5), w*.5)
            self.assertEqual(lower_medial_normal_weight(h, 0), 0)
        for edge in (.05, .23):
            e = 1e-6
            self.assertLess(abs(lower_medial_normal_weight(edge+e, 1)-lower_medial_normal_weight(edge-e, 1))/(2*e), 1e-6)

    def test_source_contract_and_zero_identity(self):
        self.assertEqual(adapt_pipeline('original', 0), 'original')
        with self.assertRaises(ValueError):
            adapt_pipeline('original', 1)
        source = '''        region_normals = {}
                normals[index] += poly.normal * poly.area
        for normal in normals:
                if index in ear_root_ids:
                    normal = normal.lerp'''
        result = adapt_pipeline(source, 1)
        self.assertIn('skin_key = (index, region)', result)
        self.assertIn('if poly.use_smooth and region in (1, 2) and side != 0:', result)
        self.assertNotIn('normals_split_custom_set', result)
        self.assertIn('target.length > 1e-12', result)


if __name__ == '__main__':
    unittest.main()
