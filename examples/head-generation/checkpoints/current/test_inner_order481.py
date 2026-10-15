import json,unittest
from pathlib import Path
class InnerOrder481(unittest.TestCase):
 def test_main_inner_tip_is_shorter_than_lower_fork(self):
  s=json.loads(Path(__file__).with_name('face_style.json').read_text())
  self.assertGreater(s['upper_lash_rails'][0][0],min(t for t,y in s['inner_lash_fork_outline']))
  self.assertEqual(s['upper_lash_rails'][-1],[1.1,.048,.048])
  self.assertAlmostEqual(s['accent_sheet_slope'],.35)
