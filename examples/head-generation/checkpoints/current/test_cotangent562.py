import unittest
from pathlib import Path
class LocalWeights(unittest.TestCase):
 def test_actual_geometric_weights_and_limits(self):
  s=((Path(__file__).parent/"ear_boundary.py").read_text()+(Path(__file__).parent/"ear_surface.py").read_text())
  self.assertIn("u.dot(v)/max(u.cross(v).length,1e-12)",s)
  self.assertIn("limit=.003 if i in patch else ((.001 if i in moving_roots else .002)+.002*extra)",s)
  self.assertIn("if total>1e-12 else",s)

 def test_extra_stays_posterior_and_tapers(self):
  def extra(depth,height):
   t=max(0.,min(1.,(depth-.12)/.08,(height-.12)/.05,(.38-height)/.05))
   return t*t*(3-2*t)
  for d in (0.,.08,.12): self.assertEqual(extra(d,.25),0)
  for h in (.1,.12,.38,.4): self.assertEqual(extra(.25,h),0)
  self.assertEqual(extra(.25,.25),1)
  self.assertTrue(0<extra(.16,.25)<1)
