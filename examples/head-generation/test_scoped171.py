import unittest,math
from test_style_math import pure_function,STYLE
class ScopedDetailTests(unittest.TestCase):
 def test_ear_pigment_is_finite_bounded_and_lower_lobe_clear(self):
  fn,_=pure_function('textures.py','ear_color',{'STYLE':STYLE,'SKIN':STYLE['skin_rgb']})
  for i in range(51):
   for j in range(51):
    color=fn(i/50,j/50)
    self.assertTrue(all(math.isfinite(x) and 0<=x<=1 for x in color))
   self.assertEqual(fn(i/50,.15)[:3],tuple(STYLE['skin_rgb']))
 def test_new_ear_details_make_localized_nonzero_changes(self):
  fn,_=pure_function('textures.py','ear_color',{'STYLE':STYLE,'SKIN':STYLE['skin_rgb']})
  prior=dict(STYLE,ear_helix_highlight_strength=0,ear_lobe_curl_strength=0)
  base,_=pure_function('textures.py','ear_color',{'STYLE':prior,'SKIN':STYLE['skin_rgb']})
  diffs=[max(abs(a-b) for a,b in zip(fn(i/50,j/50),base(i/50,j/50))) for i in range(51) for j in range(51)]
  self.assertGreater(max(diffs),.02)
  self.assertLess(max(diffs),.08)
