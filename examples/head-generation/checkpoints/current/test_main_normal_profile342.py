import unittest
from test_spatial_lashes196 import STYLE,S
class NormalProfileTests(unittest.TestCase):
 def test_profile_is_bounded_and_tips_stay_crisp(self):
  p=STYLE["accent_main_normal_blend_profile"]
  for i in range(201):
   v=S["curve"](-1.15+2.25*i/200,1,p);self.assertGreaterEqual(v,0);self.assertLessEqual(v,1)
  self.assertTrue(all(weight == 0.0 for _, weight in p))
  self.assertEqual(S["curve"](0,1,p),0.0)
