import unittest
from test_style_math import STYLE,pure_function
# Preserve the original candidate394 regression fixture.
C=dict(STYLE['calibration'],ear_lower_posterior_sweep=.045)
class EarLowerSweep394(unittest.TestCase):
 def test_root_and_width_preserved_with_less_lower_posterior_bulge(self):
  curve,_=pure_function('face.py','curve',{})
  fn,_=pure_function('ears.py','membrane_offset',{'curve':curve})
  previous=dict(C,ear_lower_posterior_sweep=.085)
  for t in [.03,.08,.15,.25,.4,.6,.8,.95]:
   for u in [0,.22,.5,.7,.88,1]:
    old=fn(t,u,previous,-.33);new=fn(t,u,C,-.33)
    self.assertAlmostEqual(new[0],old[0])
    self.assertGreaterEqual(new[1],0)
    self.assertLessEqual(new[1],old[1]+1e-12)
    self.assertLessEqual((old[1]-new[1])*.236,.0045)
    if u==0:self.assertEqual(new,old)

class EarLowerSweep497(unittest.TestCase):
 def test_bounded_forward_adjustment_with_unchanged_root_and_width(self):
  curve,_=pure_function('face.py','curve',{})
  fn,_=pure_function('ears.py','membrane_offset',{'curve':curve})
  current=STYLE['calibration'];self.assertEqual(current['ear_lower_posterior_sweep'],.035)
  for i in range(101):
   t=i/100
   for u in [0,.22,.5,.7,.88,1]:
    old=fn(t,u,C,-.33);new=fn(t,u,current,-.33)
    self.assertAlmostEqual(old[0],new[0])
    self.assertGreaterEqual(new[1],0)
    self.assertGreaterEqual(old[1]-new[1],-1e-12)
    self.assertLessEqual((old[1]-new[1])*.236,.0012)
    if u==0:self.assertEqual(old,new)
