import unittest,json
from pathlib import Path
class LeafSweep426(unittest.TestCase):
 def test_declared_leaf_space(self):
  s=json.loads(Path(__file__).with_name('face_style.json').read_text());p=s['upper_swept_tuft_outline']
  self.assertAlmostEqual(min(p)[1],.0265)  #482 broad attached root edge
  self.assertAlmostEqual(max(p)[0],.57)
  self.assertAlmostEqual(max(p)[1],.0637)
  self.assertAlmostEqual(s['accent_tuft_flare_m'],.009350001886579574)  #439 compensates parent pitch; free tip retained

 def test_inner_span_updated_leaf_tip481(self):
  from test_spatial_lashes196 import S,STYLE
  k="upper_swept_tuft_outline";tip=max(STYLE[k],key=lambda p:p[1])
  q=S["upper_accent_point"](1,*tip,0,k)
  self.assertLess(abs(q[1]+.08782227839098614),.003)  # Allow bounded parent-rail redesign; no unbounded free-tip shift
