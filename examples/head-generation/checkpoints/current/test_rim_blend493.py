import unittest
from ear_boundary import blend_rim_displacements
class HarmonicRim496(unittest.TestCase):
 def test_linear_harmonic_extension_and_fixed_end(self):
  p=[(i,0,0) for i in range(8)];r=list(p);r[0]=(0,.006,0)
  n={i:{j for j in [i-1,i+1] if 0<=j<8} for i in range(8)}
  out,changed=blend_rim_displacements(p,r,n,{0},set(range(1,7)))
  self.assertEqual(changed,set(range(1,7)))
  for i in range(8):self.assertAlmostEqual(out[i][1],.006*(1-i/7),places=7)
  self.assertEqual(out[7],p[7]);self.assertEqual(out[0],r[0])
 def test_no_unpermitted_vertices(self):
  p=[(0,0,0),(1,0,0),(2,0,0)];r=[(0,.006,0),p[1],p[2]]
  out,changed=blend_rim_displacements(p,r,{0:{1},1:{0,2},2:{1}},{0},set())
  self.assertFalse(changed);self.assertEqual(out,r)
 def test_displacement_maximum_principle(self):
  p=[(0,0,0),(.1,0,0),(1,0,0),(2,0,0)];r=list(p);r[0]=(0,.006,0)
  out,_=blend_rim_displacements(p,r,{0:{1},1:{0,2},2:{1,3},3:{2}},{0},{1,2})
  self.assertTrue(all(0<=q[1]<=.006 for q in out));self.assertEqual(out[3],p[3])
