import unittest
from test_style_math import pure_function

weight, _ = pure_function('eye_accents.py', 'main_root_shift_weight', {})

class PointedRootTests(unittest.TestCase):
    def test_collapsed_tip_has_continuous_root_correction(self):
        self.assertEqual(weight(.04, .04, .04), 1.0)
        for width in [1e-4, 1e-6, 1e-8]:
            for fraction in [0, .25, .5, .75, 1]:
                self.assertLessEqual(abs(1-weight(.04+width*fraction,.04,.04+width)),width/.006+1e-10)

    def test_broad_sheet_keeps_root_and_free_margin_distinct(self):
        self.assertEqual(weight(.02,.02,.04),0.0)
        self.assertEqual(weight(.04,.02,.04),1.0)
        for i in range(101):
            value=weight(.02+.02*i/100,.02,.04)
            self.assertGreaterEqual(value,0.0)
            self.assertLessEqual(value,1.0)
