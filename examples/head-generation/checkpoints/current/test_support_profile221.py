import unittest
from test_style_math import pure_function

curve, _ = pure_function('face.py','curve',{})
extend, _ = pure_function('eye_accents.py','extend_root_shifts',{'curve':curve})

class SupportProfileTests(unittest.TestCase):
    def test_missing_free_ends_continue_nearest_valid_support(self):
        self.assertEqual(extend([[0,None],[1,-.003],[2,-.004],[3,None]]),[[0,-.003],[1,-.003],[2,-.004],[3,-.004]])

    def test_actual_zero_correction_is_preserved(self):
        rows=extend([[0,-.003],[1,0.0],[2,None]])
        self.assertEqual(rows[1][1],0.0)
        self.assertEqual(rows[2][1],0.0)

    def test_interior_gap_stays_bounded_by_support(self):
        rows=extend([[0,-.003],[.5,None],[1,-.004]])
        self.assertLessEqual(rows[1][1],-.003)
        self.assertGreaterEqual(rows[1][1],-.004)
