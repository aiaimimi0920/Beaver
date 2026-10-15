import unittest
from ear_shoulder import smooth,shoulder_depth,ROWS
class EarShoulder(unittest.TestCase):
 def test_smooth_bound(self):
  self.assertEqual(smooth(-1),0);self.assertEqual(smooth(2),1)
 def test_finite_semantic_depth(self):
  for h,w,ds in ROWS:
   z,r=shoulder_depth(h,w*.9);self.assertTrue(-.2<z<.3);self.assertAlmostEqual(r,.9)
 def test_continuity(self):
  for h,w,ds in ROWS[1:-1]:
   a,_=shoulder_depth(h-1e-7,w*.9);b,_=shoulder_depth(h+1e-7,w*.9);self.assertLess(abs(a-b),1e-5)
