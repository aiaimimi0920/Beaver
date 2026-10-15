import unittest
from test_style_math import pure_function
class EarSilhouette613(unittest.TestCase):
 def test_bounded_smooth_local_depth(self):
  bump,_=pure_function('ear_silhouette.py','bump',{})
  fn,_=pure_function('ear_silhouette.py','silhouette_depth',{'bump':bump})
  self.assertAlmostEqual(fn(1.607),.0025)
  self.assertLess(fn(1.641),0)
  for h in [1.55,1.58,1.655,1.7]:self.assertAlmostEqual(fn(h),0)
  for i in range(1001):self.assertLessEqual(abs(fn(1.55+i*.0002)),.0025)
