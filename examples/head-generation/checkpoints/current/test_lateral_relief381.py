import unittest
from test_spatial_lashes196 import STYLE
class LateralRelief381(unittest.TestCase):
 def test_fold_has_bounded_relief(self):
  self.assertGreater(STYLE["accent_lateral_roll_m"],.00016)
  self.assertLessEqual(STYLE["accent_lateral_roll_m"],.0008)
 def test_rejected_companion_gap_is_reverted(self):
  self.assertTrue(0.0 <= STYLE["accent_lateral_branch_separation_H"] <= .003)
