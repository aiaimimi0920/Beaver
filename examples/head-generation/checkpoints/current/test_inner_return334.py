import unittest
from test_spatial_lashes196 import S,STYLE
class InwardReturnTests(unittest.TestCase):
 def test_root_fixed_tip_recedes_without_changing_outline(self):
  key="inner_lash_fork_outline"; value=STYLE["accent_inner_fork_lower_recede_m"]
  try:
   for side in (-1,1):
    root=max(STYLE[key],key=lambda p:p[1]); tip=min(STYLE[key],key=lambda p:p[1])
    STYLE["accent_inner_fork_lower_recede_m"]=0
    before=[S["upper_accent_point"](side,t,h,0,key) for t,h in (root,tip)]
    STYLE["accent_inner_fork_lower_recede_m"]=value
    after=[S["upper_accent_point"](side,t,h,0,key) for t,h in (root,tip)]
    self.assertEqual(before[0],after[0])
    self.assertAlmostEqual(after[1][1]-before[1][1],value)
    self.assertEqual(after[1][0],before[1][0]); self.assertEqual(after[1][2],before[1][2])
  finally: STYLE["accent_inner_fork_lower_recede_m"]=value
