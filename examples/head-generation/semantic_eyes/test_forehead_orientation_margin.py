import unittest
import numpy as np
from forehead_orientation_margin import forehead_orientation_margin_targets as f, forehead_signed_chart_cross_mm2 as cross
class Tests(unittest.TestCase):
 def call(self,**kw):
  args=dict(targets=[0.,0.],rows=[1],raw_per_scaled=[1.,2.],smooth_mm2=[1e-6],rounded_mm2=[-3.1e-4],variable_rows=[True]);args.update(kw);return f(**args)
 def test_units(self):self.assertAlmostEqual(self.call()[0][1],.000411/2)
 def test_rows(self):
  for r in ([1.0],[-1],[2],[True]):
   with self.assertRaises(ValueError):self.call(rows=r)
  with self.assertRaises(ValueError):f([0,0],[1,1],[1,1],[1,1],[1,1],[True,True])
 def test_finite_and_shape(self):
  for kw in ({'targets':[0,np.nan]},{'safety_mm2':np.nan},{'safety_mm2':0},{'raw_per_scaled':[1,0]},{'smooth_mm2':[[1]]},{'variable_rows':[1]},{'rounded_mm2':[np.inf]}):
   with self.assertRaises(ValueError):self.call(**kw)
 def test_fixed(self):
  with self.assertRaises(ValueError):self.call(variable_rows=[False])
 def test_floor(self):
  t,e=self.call(smooth_mm2=[.2],rounded_mm2=[.2],accepted_mm2=[.3],interior_cap_mm2=.001);self.assertEqual(t[1],.0005)
  t,e=self.call(smooth_mm2=[.2],rounded_mm2=[.2],variable_rows=[False],accepted_mm2=[.3],interior_cap_mm2=.001);self.assertEqual(t[1],0)
 def test_retained(self):self.assertEqual(self.call(targets=[0,2])[0][1],2)
 def test_radius(self):
  p=np.array([[0,-1,0],[.1,-1,0],[.1,-1,.2],[0,-1,.2]])
  a=cross(p,[[0,1,2,3]],np.ones((1,4)),.1)
  np.testing.assert_allclose(cross(p,[[0,1,2,3]],np.ones((1,4)),.2),2*a)
  self.assertTrue((a>0).all())
 def test_geometry_validation(self):
  for q in ([[0.,1,2,3]],[[0,1,2,4]],[[0,1,2,2]]):
   with self.assertRaises(ValueError):cross(np.ones((4,3)),q,np.ones((1,4)),.1)
if __name__=='__main__':unittest.main()
