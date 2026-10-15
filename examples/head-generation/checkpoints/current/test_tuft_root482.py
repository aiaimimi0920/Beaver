import unittest,json
from pathlib import Path
from test_style_math import pure_function
class BroadRoot482(unittest.TestCase):
 def test_root_is_an_edge_and_tip_is_a_point(self):
  s=json.loads(Path(__file__).with_name("face_style.json").read_text())
  rows=pure_function("eye_accents.py","outline_sections",{"STYLE":s})[0](s["upper_swept_tuft_outline"])
  self.assertAlmostEqual(rows[0][0],.08)
  self.assertAlmostEqual(rows[0][2]-rows[0][1],.024)
  self.assertAlmostEqual(rows[-1][2]-rows[-1][1],0)
  self.assertTrue(all(hi>=lo for _,lo,hi in rows))
