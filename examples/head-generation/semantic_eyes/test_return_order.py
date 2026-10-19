"""Stable original topology IDs, never coordinate or object-hash ordering."""
from pathlib import Path
import unittest
from source_return_order import adapt_return_order


class ReturnOrderTests(unittest.TestCase):
    def test_adaptation_preserves_geometry_and_guards(self):
        source = (Path(__file__).parents[1] / 'checkpoints/current/ear_return.py').read_text()
        changed = adapt_return_order(source)
        compile(changed, 'ear_return.py', 'exec')
        self.assertIn('bm.verts.index_update()', changed)
        self.assertIn('key=original_index.__getitem__', changed)
        self.assertIn('selected.sort(key=', changed)
        self.assertEqual(source.count('assert '), changed.count('assert '))
        for line in source.splitlines():
            if any(key in line for key in ('point.', 'width =', 'new_loop[', 'face.normal', 'face.calc_area')):
                self.assertIn(line, changed)
        with self.assertRaises(ValueError):
            adapt_return_order(changed)

    def test_source_contract_failure_is_closed(self):
        with self.assertRaises(ValueError):
            adapt_return_order('def changed_recipe(): pass')

    def test_creation_order_ignores_object_hash_and_edge_iteration(self):
        import ast
        import random
        from types import SimpleNamespace
        source = (Path(__file__).parents[1] / 'checkpoints/current/ear_return.py').read_text()
        function = next(n for n in ast.parse(adapt_return_order(source)).body
                        if isinstance(n, ast.FunctionDef) and n.name == 'add_final_return')
        statements = [n for n in function.body if
            (isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and
             t.id in ('original_index', 'ids') for t in n.targets)) or
            (isinstance(n, ast.Expr) and isinstance(n.value, ast.Call) and
             isinstance(n.value.func, ast.Attribute) and n.value.func.attr == 'sort')]
        code = compile(ast.Module(body=statements, type_ignores=[]), 'ordering', 'exec')
        class Vertex:
            def __init__(self, index, token):
                self.index, self.token = index, token
            def __hash__(self):
                return self.token
        for seed in range(20):
            rng = random.Random(seed)
            vertices = [Vertex(i, rng.randrange(1000000)) for i in range(6)]
            edges = [SimpleNamespace(verts=(vertices[a], vertices[b]))
                     for a,b in [(4,5),(0,1),(2,3),(1,2)]]
            rng.shuffle(edges)
            scope = {'bm':SimpleNamespace(verts=vertices), 'selected':edges}
            exec(code, scope)
            self.assertEqual([v.index for v in scope['ids']], list(range(6)))
            self.assertEqual([tuple(sorted(v.index for v in e.verts))
                              for e in scope['selected']], [(0,1),(1,2),(2,3),(4,5)])
