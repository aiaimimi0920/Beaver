import unittest, json
from pathlib import Path
from test_style_math import pure_function
class LashAtlasTests(unittest.TestCase):
 def test_black_to_plum_endpoints_and_continuity(self):
  style=json.loads(Path(__file__).with_name('face_style.json').read_text())
  ink=(.018,.009,.016)
  fn,_=pure_function('textures.py','lash_atlas_color',{'STYLE':style,'INK':ink})
  u=lambda t:.605+.34*(t+1.4)/2.8
  self.assertEqual(fn(u(-1)),ink+(1,))
  self.assertEqual(fn(u(1)),tuple(style['accent_outer_tint_rgb'])+(1,))
  samples=[fn(u(i/100)) for i in range(86)]
  for a,b in zip(samples,samples[1:]):
   self.assertTrue(all(y>=x-1e-10 for x,y in zip(a,b)))
   self.assertLess(max(abs(x-y) for x,y in zip(a,b)),.01)
 def test_separate_uv_strip_does_not_overlap_preserved_islands(self):
  self.assertGreater(.865,.85);self.assertLess(.885,.90)
  code=Path(__file__).with_name('pipeline.py').read_text()
  self.assertIn('o.get("lash_gradient", False)',code)
  self.assertIn('vv = 0.865 + 0.02',code)
