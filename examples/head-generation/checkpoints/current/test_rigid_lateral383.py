import unittest
from pathlib import Path
class RigidLateral383(unittest.TestCase):
 def test_clearance_uses_whole_lateral_sections_and_keeps_bound(self):
  s=Path(__file__).with_name('eye_accents.py').read_text()
  self.assertIn("counts=[1 if hi-lo<1e-9 else 6",s)
  self.assertIn("counts=[1 if f in (0.0,1.0) else 6 for f in companion_stations(STYLE)]",s)
  from lateral_fit import companion_stations
  self.assertEqual(companion_stations({}),tuple(i/16 for i in range(17)))
  self.assertIn("maximum_facet_shift < .001",s)
  self.assertIn("for group in groups.values():",s)
