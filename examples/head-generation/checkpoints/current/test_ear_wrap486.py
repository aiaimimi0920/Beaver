import unittest,math
from test_style_math import STYLE,pure_function
class EarWrap486(unittest.TestCase):
 def test_only_lower_return_gets_more_medial_wrap(self):
  c=STYLE['calibration'];curve,_=pure_function('face.py','curve',{});fn,_=pure_function('ears.py','membrane_offset',{'curve':curve})
  maximum=0
  for i in range(1,1000):
   t=i/1000;out,_=fn(t,1,c,-.33);profile=curve(t,1,c['ear_return_width_profile'])
   old=min(c['ear_return_lip_width']*math.sin(math.pi*t),out*min(1.15,profile));new=min(c['ear_return_lip_width']*math.sin(math.pi*t),out*min(1.4,profile))
   self.assertGreaterEqual(new,old);maximum=max(maximum,(new-old)*c['height_m'])
   if t>=.7:self.assertEqual(old,new)
  self.assertGreater(maximum,.001);self.assertLess(maximum,.005)
