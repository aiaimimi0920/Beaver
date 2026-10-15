import unittest
from test_spatial_lashes196 import S,STYLE
from main_curve import curved_section,is_collinear_refinement
class Recovery473(unittest.TestCase):
 def test_free_bow_boundaries(self):
  f=S['tuft_free_bow']
  for p in [0,.1,.25,1]:self.assertAlmostEqual(f(p,.0015),0,places=12)
  self.assertAlmostEqual(f(.625,.0015),.0015)
  for i in range(101):self.assertTrue(0<=f(i/100,.0015)<=.0015)
 def test_fold_root_taper(self):
  f=S['tuft_fold_relief'];self.assertEqual(f(.08),0)
  self.assertAlmostEqual(f(.57),.0005)
  vals=[f(.08+.49*i/100) for i in range(101)]
  self.assertEqual(vals,sorted(vals));self.assertTrue(all(0<=v<=.0005 for v in vals))
 def test_curved_sections_preserve_boundaries(self):
  args=((0,0,0),(0,-.003,.0045),(0,0,.01),.00018,'creased')
  a=curved_section(*args);b=curved_section(*args,.25)
  for i in (1,3,5):self.assertEqual(a[i],b[i])
  for i,j in ((1,0),(2,9),(3,8),(4,7),(5,6)):self.assertAlmostEqual(b[j][1]-b[i][1],.00018)
  for r,expected in ((a,True),(b,False)):
   for i,j,k in ((2,1,3),(4,3,5)):
    f=(r[i][2]-r[j][2])/(r[k][2]-r[j][2]);self.assertEqual(is_collinear_refinement(r[i],r[j],r[k],f),expected)
 def test_frozen_geometry_guards(self):
  self.assertEqual(STYLE['accent_evaluated_clearance_m'],.00035)
  self.assertEqual(STYLE['accent_band_thickness_m'],.00018)
