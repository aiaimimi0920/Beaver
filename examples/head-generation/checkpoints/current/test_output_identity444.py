import unittest,ast,json
from pathlib import Path
class OutputIdentity(unittest.TestCase):
 def test_output_path_matches_recipe(self):
  root=Path(__file__).parent
  tree=ast.parse((root/'pipeline.py').read_text())
  out=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='OUT' for t in n.targets))
  style=json.loads((root/'face_style.json').read_text())
  self.assertEqual(out,'assets/aster/'+style['preset']+'/')
