import unittest
from pathlib import Path
class FairingOrder522(unittest.TestCase):
 def test_width_before_final_fairing(self):
  s=(Path(__file__).parent/"pipeline.py").read_text()
  self.assertLess(s.index("= fit_ear_under_width("),s.index("= fair_ear_jaw_rim("))
