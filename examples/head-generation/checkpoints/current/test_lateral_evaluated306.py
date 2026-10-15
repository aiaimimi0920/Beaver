import unittest
from types import SimpleNamespace
from test_spatial_lashes196 import S, STYLE
from test_style_math import pure_function

class LateralEvaluatedTests(unittest.TestCase):
 def test_near_final_surface_is_used_at_actual_station(self):
  x,y=.075,1.53; base=S['depth'](x,y); rays=[]
  class Tree:
   def ray_cast(self,start,*args):
    rays.append(start);return SimpleNamespace(y=-(base-.002)),None,None,None
  fn,_=pure_function('eye_accents.py','lateral_attached_depth',dict(S,Vector=lambda p:p,_EVALUATED_LID_SUPPORT=Tree()))
  self.assertAlmostEqual(fn(x,y),base-.002)
  self.assertEqual(rays[0],(x,-.5,y))
 def test_distant_surface_is_rejected(self):
  class Tree:
   def ray_cast(self,*args):return SimpleNamespace(y=.3),None,None,None
  fn,_=pure_function('eye_accents.py','lateral_attached_depth',dict(S,Vector=lambda p:p,_EVALUATED_LID_SUPPORT=Tree()))
  self.assertAlmostEqual(fn(.075,1.53),S['depth'](.075,1.53))

 def test_cross_section_uses_shared_anchor_not_free_tooth_ray(self):
  x,anchor,y=.079,.075,1.53; base=S['depth'](x,y); anchor_base=S['depth'](anchor,y); rays=[]
  class Tree:
   def ray_cast(self,start,*args):
    rays.append(start);return SimpleNamespace(y=-(anchor_base-.001)),None,None,None
  fn,_=pure_function('eye_accents.py','lateral_attached_depth',dict(S,Vector=lambda p:p,_EVALUATED_LID_SUPPORT=Tree()))
  self.assertAlmostEqual(fn(x,y,anchor),base-.001)
  self.assertEqual(rays[0],(anchor,-.5,y))
