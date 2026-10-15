"""Root-to-ridge preservation; free margin is no longer translated as a rigid shelf."""
import unittest,math
from test_style_math import pure_function
from test_spatial_lashes196 import S, STYLE
weight,_=pure_function('eye_accents.py','fold_support_shift_weight',S)
class SupportSectionTests(unittest.TestCase):
 def test_attached_facet_preservation_is_full(self):
  self.assertEqual(STYLE['accent_support_section_preservation'],1.0)
  lo,hi=.01,.05;ridge=STYLE['accent_fold_ridge_fraction'];crest=lo+(hi-lo)*(ridge+STYLE['accent_cross_arch_ratio']*math.sin(math.pi*ridge))
  self.assertAlmostEqual(weight(hi,lo,hi),1);self.assertAlmostEqual(weight(crest,lo,hi),1)
 def test_free_margin_correction_is_bounded_and_monotonic(self):
  lo,hi=.01,.05;values=[weight(lo+(hi-lo)*i/100,lo,hi) for i in range(101)]
  self.assertAlmostEqual(values[0],STYLE['accent_free_margin_support_fraction'])
  self.assertTrue(all(0<=v<=1 for v in values));self.assertTrue(all(a<=b for a,b in zip(values,values[1:])))
 def test_pointed_caps_follow_root_to_avoid_splits(self):
  self.assertEqual(weight(.03,.03,.03),1)
