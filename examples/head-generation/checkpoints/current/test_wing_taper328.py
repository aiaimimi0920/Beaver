import unittest
from test_style_math import STYLE
class WingTaperTests(unittest.TestCase):
 def test_declared_smooth_drop_preserves_section_and_root(self):
  r=STYLE['outer_lash_wing_rails']
  baseline=[[.65,.0227,.0227],[.782,.019,.029],[.9404,.0195,.033],[1.0812,.02,.03],[1.1868,.022,.025],[1.2484,.024,.024]]
  for row,base in zip(r,baseline):
   u=(row[0]-r[0][0])/(r[-1][0]-r[0][0]);drop=.005*u*u*(3-2*u)
   self.assertAlmostEqual(row[1],base[1]-drop)
   # 649 raises only the upper guide to replace the rejected wide V opening.
   self.assertGreaterEqual(row[2],base[2]-drop)
  self.assertEqual(r[0],baseline[0])
  self.assertEqual(r[0][1],r[0][2]);self.assertEqual(r[-1][1],r[-1][2]);self.assertTrue(all(p[2]>p[1] for p in r[1:-1]))
