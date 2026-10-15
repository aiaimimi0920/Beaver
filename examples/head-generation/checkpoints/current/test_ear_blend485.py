import unittest
from ear_boundary import fair_chain
class EarBlend485(unittest.TestCase):
 def test_smooth_fade_without_expanding_region(self):
  pts=[(i*.002,(.001 if i%2 else 0),0) for i in range(9)]
  ns={i:[i-1,i+1] for i in range(1,8)};sel=set(range(1,8))
  raw=fair_chain(pts,ns,sel);blend=fair_chain(pts,ns,sel,blend_rings=3)
  self.assertEqual(blend[0],pts[0]);self.assertEqual(blend[-1],pts[-1])
  for i in sel:
   t=min(1,min(i,8-i)/3);w=t*t*(3-2*t)
   for k in range(3):self.assertAlmostEqual(blend[i][k]-pts[i][k],w*(raw[i][k]-pts[i][k]))
