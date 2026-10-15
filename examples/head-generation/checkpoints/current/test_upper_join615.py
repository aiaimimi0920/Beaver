import unittest
from test_style_math import pure_function
class UpperJoin615(unittest.TestCase):
 def test_local_symmetric_bounded_shift(self):
  b,_=pure_function('ear_silhouette.py','bump',{})
  f,_=pure_function('ear_silhouette.py','upper_join_offset',{'bump':b})
  a=(.092,.005,1.648)
  self.assertAlmostEqual(f(a,a),-.0048)
  self.assertAlmostEqual(f((-.092,.005,1.648),a),-.0048)
  for p in [(0,.005,1.648),(.092,.04,1.648),(.092,.005,1.68)]:self.assertEqual(f(p,a),0)
