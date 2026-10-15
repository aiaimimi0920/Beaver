import unittest
from test_spatial_lashes196 import S, STYLE

class WingDepthTests(unittest.TestCase):
 def test_depth_calibration_keeps_root_and_frontal_outline(self):
  key='outer_lash_wing_rails'; old=STYLE['accent_wing_tip_recede_m']
  try:
   for side in [-1,1]:
    for index in [0,-1]:
     t,lo,hi=STYLE[key][index]
     STYLE['accent_wing_tip_recede_m']=0
     base=S['upper_accent_point'](side,t,lo,0,key)
     STYLE['accent_wing_tip_recede_m']=old
     actual=S['upper_accent_point'](side,t,lo,0,key)
     self.assertAlmostEqual(actual[0],base[0]);self.assertAlmostEqual(actual[2],base[2])
     self.assertAlmostEqual(actual[1]-base[1],0 if index==0 else old)
  finally:STYLE['accent_wing_tip_recede_m']=old
