import unittest
from test_style_math import STYLE,pure_function
class EarBasin501(unittest.TestCase):
 def test_fixed_root_rim_and_bounded_basin(self):
  curve,_=pure_function('face.py','curve',{})
  fn,_=pure_function('ears.py','membrane_offset',{'curve':curve})
  c=STYLE['calibration'];old=dict(c,ear_depth_profile=[[0,0],[.22,.32],[.7,.72],[.88,.79],[1,1]])
  for i in range(101):
   for j in range(101):
    t=i/100;u=j/100;a=fn(t,u,old,-.33);b=fn(t,u,c,-.33)
    self.assertEqual(a[0],b[0])
    self.assertLessEqual(abs(a[1]-b[1])*.236,.002)
    if j in (0,100):self.assertEqual(a,b)
