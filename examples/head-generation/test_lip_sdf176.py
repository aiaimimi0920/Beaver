import ast,math,unittest
from pathlib import Path
from test_style_math import STYLE
class LipSDFTests(unittest.TestCase):
 def functions(self):
  c=STYLE['calibration'];scope={'C':c,'H':c['height_m'],'Y0':c['origin_y_m'],'STYLE':STYLE,'exp':math.exp}
  tree=ast.parse(Path(__file__).with_name('face.py').read_text());ns=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['lip_shoulder_support','lip_detail_depth']]
  exec(compile(ast.Module(body=ns,type_ignores=[]),'lip field','exec'),scope);return scope
 def test_shadow_attenuation_preserves_geometric_detail_field(self):
  s=self.functions();fn=s['lip_detail_depth'];h=s['H'];y=s['Y0']+h*.13;e=.0002;x=h*.05
  slope=(fn(x+e,y)-fn(x-e,y))/(2*e)
  self.assertGreater(abs(slope),.01)
  scale=STYLE['lip_sdf_scale'];self.assertGreaterEqual(scale,0);self.assertLessEqual(scale,.2)
  self.assertAlmostEqual(slope-slope*(1-scale),slope*scale)
  self.assertGreater(fn(0,y),0)
 def test_detail_field_has_negligible_eye_region_contribution(self):
  s=self.functions();fn=s['lip_detail_depth'];h=s['H'];y0=s['Y0']
  for x in [-.06,-.02,0,.02,.06]:
   for v in [.35,.4,.5,.6]:self.assertLess(abs(fn(x,y0+h*v)),1e-12)
