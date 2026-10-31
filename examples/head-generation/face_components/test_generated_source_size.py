"""Check emitted sources, not only the source-generating Python modules."""
import ast
from pathlib import Path
import sys
import tempfile
import unittest
from bundle import build_plan


class SourceSizeTests(unittest.TestCase):
    def test_public_density_recipe_emits_cohesive_sources_under_limit(self):
        parent = Path(__file__).resolve().parent.parent
        sys.path.insert(0, str(parent))
        try:
            from source_layout.package import literal_writes
            from source_layout.python_lines import conservative_effective_lines
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                required = ['project.godot', 'authoring/head_generation_guide.md']
                required += ['assets/aster/head_recovery_49/' + name for name in (
                    'aster_head_editable.blend', 'aster_head.glb', 'aster_definition.tres',
                    'hair_base.png', 'hair_ilm.png')]
                for name in required:
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('CPU-only source packaging fixture')
                for eye in ('almond', 'segmented'):
                    selection = dict(eyes=eye, brows='research-angled',
                                     nose='research-balanced', mouth='research-line',
                                     ears='research-slender-oval', face='research-compact-rounded')
                    plan = build_plan(selection, 'assets/aster/general_face_eyes_combo_cpu_' + eye + '/', root)
                    tree = ast.parse(plan['generation']['code'])
                    values = {n.targets[0].id: n.value for n in tree.body
                              if isinstance(n, ast.Assign) and isinstance(n.targets[0], ast.Name)
                              and isinstance(n.value, ast.Constant) and isinstance(n.value.value, str)}

                    class ExpandLiterals(ast.NodeTransformer):
                        def visit_Name(self, node):
                            return values.get(node.id, node) if isinstance(node.ctx, ast.Load) else node

                    writes = literal_writes(ast.unparse(ExpandLiterals().visit(tree)))
                    sources = {name: value[0] for name, value in writes.items() if name.endswith('.py')}
                    sources.update(plan["source_packaging"]["helpers"])
                    self.assertGreater(len(sources), 20)
                    for name, source in sources.items():
                        compile(source, name, 'exec')
                        count = conservative_effective_lines(source)
                        self.assertLessEqual(count, 500, name)
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
