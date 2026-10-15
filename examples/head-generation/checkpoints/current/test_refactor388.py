import ast,hashlib,json,unittest
from pathlib import Path
class Refactor388(unittest.TestCase):
 def test_moved_functions_are_identical(self):
  p=Path(__file__).parent
  expected=json.loads((p/'thin-ribbon-refactor-contract388.json').read_text())
  actual={n.name:hashlib.sha256(ast.dump(n,include_attributes=False).encode()).hexdigest() for n in ast.parse((p/'thin_ribbon.py').read_text()).body if isinstance(n,ast.FunctionDef)}
  self.assertEqual(actual,expected)
 def test_modules_stay_below_soft_limit_and_report_active_profile(self):
  p=Path(__file__).parent
  for name in ['eye_accents.py','thin_ribbon.py','lateral_fit.py']:
   lines=(p/name).read_text().splitlines()
   self.assertLessEqual(sum(bool(x.strip()) and not x.lstrip().startswith('#') for x in lines),700)
  pipeline=(p/'pipeline.py').read_text()
  self.assertIn('report["main_lash_crease_normal_blend_profile"]',pipeline)
  self.assertNotIn('report["main_lash_crease_normal_blend"]',pipeline)
