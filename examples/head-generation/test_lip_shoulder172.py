import unittest
from test_style_math import pure_function,STYLE
class LipShoulderTests(unittest.TestCase):
 def test_support_is_bounded_symmetric_and_center_preserved(self):
  c=STYLE['calibration'];fn,_=pure_function('face.py','lip_shoulder_support',{'C':c,'H':1,'Y0':0})
  for y in [0,.12,.147,.17,.4,.6]:
   self.assertEqual(fn(0,y),0)
   for i in range(101):
    x=i/500
    self.assertAlmostEqual(fn(x,y),fn(-x,y))
    self.assertGreaterEqual(fn(x,y),0)
    self.assertLessEqual(fn(x,y),.003)
  self.assertGreater(fn(.03,.147),.0029)
  self.assertLess(fn(.03,.4),1e-15)
