import unittest,json
from pathlib import Path
from lateral_fit import continuous_guide_moves
class Bow389(unittest.TestCase):
 def test_reduced_curve_preserves_ends_and_height(self):
  fs=[i/10 for i in range(11)];bases=[(.08-.003*f**4,-.1+.001*f+.002*4*f*(1-f),1-f) for f in fs]
  shifts=continuous_guide_moves(fs,bases,.35)
  for f,b,d in zip(fs,bases,shifts):
   self.assertAlmostEqual(b[1]+d[1],-.1+.001*f+.0007*4*f*(1-f),12)
   self.assertEqual(d[2],0)
  self.assertEqual(shifts[0],(0,0,0));self.assertEqual(shifts[-1],(0,0,0))
 def test_parameter_is_used(self):
  p=Path(__file__).parent
  self.assertEqual(json.loads((p/'face_style.json').read_text())['accent_lateral_guide_bow_fraction'],.35)
  self.assertIn('continuous_guide_moves(stations,seated_bases,style["accent_lateral_guide_bow_fraction"],style["accent_lateral_guide_inward_power"])',(p/'lateral_fit.py').read_text())
