import unittest,ast,json
from pathlib import Path
class MainNormalBlendTests(unittest.TestCase):
 def test_blend_remains_bounded_and_front_scoped(self):
  p=Path(__file__).parent
  d=json.loads((p/"face_style.json").read_text())
  self.assertGreater(d["accent_main_crease_normal_blend"],0)
  self.assertLessEqual(d["accent_main_crease_normal_blend"],1.0)  # full ordinary area-weighted smoothing is a valid endpoint
  s=(p/"pipeline.py").read_text();ast.parse(s)
  self.assertIn('part.name.startswith("Upper lash ")',s)
  self.assertIn('accent_zones.data[poly.index].value in (1,2)',s)
  self.assertIn('front_key = (index, region % 1000)',s)
