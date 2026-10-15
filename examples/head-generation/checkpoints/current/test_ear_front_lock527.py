import unittest
from pathlib import Path
class FrontLock527(unittest.TestCase):
 def test_final_rim_preserves_lateral_and_height(self):
  s=Path(__file__).with_name("ear_boundary.py").read_text()
  self.assertIn("result=[(preserved[i][0],point[1],preserved[i][2]) for i,point in enumerate(result)]",s)
