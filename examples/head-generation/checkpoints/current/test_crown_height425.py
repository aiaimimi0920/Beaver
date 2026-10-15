import unittest,json
from pathlib import Path
class CrownHeight425(unittest.TestCase):
 def test_crown_is_interior_and_relief_retained(self):
  s=json.loads(Path(__file__).with_name('face_style.json').read_text())
  self.assertEqual(s['accent_final_crown_height_fraction'],.45)
  self.assertTrue(.001 <= s['accent_final_crown_relief_m'] <= .003)
  self.assertEqual(s['accent_free_surface_follow'],0)
 def test_front_and_return_height_move_together(self):
  source=Path(__file__).with_name('accent_attachment.py').read_text()
  self.assertIn('for v in crest: v.co.z=low[0].co.z+(high[0].co.z-low[0].co.z)*q',source)
