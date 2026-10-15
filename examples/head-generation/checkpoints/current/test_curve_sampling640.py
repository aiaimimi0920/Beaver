import ast,unittest,json
from ast_contract import canonical_function_dump
from test_accent_attachment_refactor import combined_accent_tree
from pathlib import Path
from lateral_fit import paired_layer_delta
class SamplingScopeTests(unittest.TestCase):
 def test_only_side_sampling_changes_in_accent_builder(self):
  root=Path(__file__).parent
  def normalized(p):
   s=p.read_text().replace('        from side_leaf import build_side_leaves\n        build_side_leaves(PARTS, lateral_sections(), STYLE)\n','');t=combined_accent_tree(ast.parse(s));t.body=[n for n in t.body if not(isinstance(n,ast.FunctionDef) and n.name=='lateral_sections')];return ast.dump(t)
  self.assertEqual(canonical_function_dump(normalized(root/'eye_accents.py')),canonical_function_dump(json.loads((root/'baseline_fixtures.json').read_text())['617']['accent_ast']))
 def test_companion_follows_parent_height_not_parallel_offset(self):
  e=(.074,-.08,1.640);t=(.074,-.08,1.641);d=paired_layer_delta(e,t,1)
  self.assertAlmostEqual(e[2]+d[2],t[2]);self.assertAlmostEqual(d[2],.001)
