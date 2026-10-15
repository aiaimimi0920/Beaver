import unittest,json
from pathlib import Path
from lateral_fit import continuous_guide_moves
class LateralArc393(unittest.TestCase):
 def test_quadratic_arc_is_mirrored_continuous_and_fixed_at_ends(self):
  style=json.loads((Path(__file__).parent/'face_style.json').read_text());self.assertEqual(style['accent_lateral_guide_inward_power'],3.0)
  fs=[i/20 for i in range(21)]
  for side in (-1,1):
   bases=[(side*(.08-.003*f),-.1,1-f) for f in fs]
   moves=continuous_guide_moves(fs,bases,.35,style['accent_lateral_guide_inward_power'])
   xs=[side*(b[0]+d[0]) for b,d in zip(bases,moves)]
   self.assertTrue(all(a>=b for a,b in zip(xs,xs[1:])))
   self.assertAlmostEqual(xs[10],.08-.003*.125)
   self.assertEqual(moves[0],(0,0,0));self.assertEqual(moves[-1],(0,0,0))
