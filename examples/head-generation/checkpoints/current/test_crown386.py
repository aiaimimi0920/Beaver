import unittest,json
from pathlib import Path
class Crown386(unittest.TestCase):
 def test_bounded_crown_with_protected_guides(self):
  s=json.loads(Path(__file__).with_name('face_style.json').read_text())
  self.assertTrue(.001 <= s['accent_final_crown_relief_m'] <= .003)  # candidate475 declared fold relief, not a collision threshold
  self.assertEqual(s['accent_final_max_hinge_shift_m'],.004)
  self.assertEqual(s['accent_lateral_roll_m'],.0008)
