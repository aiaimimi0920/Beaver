"""Read-only source binding: no source execution, BMesh or native dependency."""
import ast
import hashlib
import json
from pathlib import Path
import unittest

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]/'semantic_eyes'


class SourceContractTests(unittest.TestCase):
    def test_exact_source_function_contracts(self):
        contracts={'source_flow_ear_layout': {'path': 'ear_flow_geometry.py', 'function_sha256': '676c4b1879b0f909fbe96a5207ed63f2cb70bfb61273a3c477e032b83c18ccf2'}, 'flow_reconnect_skin': {'path': 'ear_flow_mesh.py', 'function_sha256': '599d1f5542c10c6127f675346703963c222fd20a1b4a178fe90c9855903d0c8d'}, 'apply_final_ear': {'path': 'ear_final_mesh.py', 'function_sha256': '75b270671e27f16f1737d46a312da86ad3621af1a5fef7b3146c72ec5d7656fc'}}
        for name,contract in contracts.items():
            source=(ROOT/contract['path']).read_text()
            function=next(n for n in ast.parse(source).body
                          if isinstance(n,ast.FunctionDef) and n.name==name)
            text=''.join(source.splitlines(keepends=True)[function.lineno-1:function.end_lineno])
            self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),contract['function_sha256'])

    def test_skin_callback_operands_and_template(self):
        p=ROOT/'ear_flow_mesh.py'
        source=p.read_text()
        self.assertIn('R,L,n = original[39],original[27],generated[41]',source)
        self.assertIn('parents = [R,L,I,J,K,U]',source)
        self.assertIn('A,B = [I,J,K],[L,n,R,s,U]',source)
        self.assertIn('left,middle,right = inside',source)
        function=next(n for n in ast.parse(source).body
                      if isinstance(n,ast.FunctionDef) and n.name=='flow_reconnect_skin')
        assignment=next(n for n in ast.walk(function) if isinstance(n,ast.Assign)
                        and any(isinstance(t,ast.Name) and t.id=='quads' for t in n.targets))
        actual=[[v.id for v in row.elts]for row in assignment.value.elts]
        self.assertEqual(actual,[['a0','a1','middle','left'],['middle','b2','b1','left'],
                                 ['b1','b0','a0','left'],['a1','a2','right','middle'],
                                 ['middle','right','b3','b2'],['right','a2','b4','b3']])


if __name__=='__main__':
    unittest.main()
