import unittest,math
from test_style_math import STYLE,pure_function
class EarEase484(unittest.TestCase):
 def test_local_sweep_and_fixed_root(self):
  c=STYLE['calibration'];curve,_=pure_function('face.py','curve',{});fn,_=pure_function('ears.py','membrane_offset',{'curve':curve})
  old=dict(c,ear_lower_sweep_ease_end=1e-9)
  for i in range(1,1001):
   t=i/1000
   a=fn(t,1,old,-.33);b=fn(t,1,c,-.33)
   self.assertEqual(a[0],b[0]);self.assertLessEqual(abs(a[1]-b[1])*.236,.0045)
   if t>=.12:self.assertEqual(a,b)
   self.assertEqual(fn(t,0,c,-.33),(0,0))
