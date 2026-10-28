"""Explicit source-flow packaging contracts; no Blender or visual acceptance."""
import ast
from pathlib import Path
import sys
import tempfile
import unittest
from bundle import build_plan

PARENT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PARENT))
from source_layout.package import literal_writes
from source_layout.layout import reconstructed
from source_layout.python_lines import conservative_effective_lines

SELECTION = dict(eyes='almond', brows='research-angled', nose='research-balanced',
                 mouth='research-line', ears='research-slender-oval', face='research-compact-rounded')
FLOW = dict(sampling_feature_flow=1, sampling_density_layout=1,
            sampling_shell_columns=13, sampling_detail_budget=1,
            sampling_surface_subdivision=0, sampling_surface_fairing=0,
            sampling_outline_refinement=0)


def fixture_root(root):
    required = ['project.godot', 'authoring/head_generation_guide.md']
    required += ['assets/aster/head_recovery_49/' + name for name in (
        'aster_head_editable.blend', 'aster_head.glb', 'aster_definition.tres', 'hair_base.png', 'hair_ilm.png')]
    for name in required:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('CPU-only source packaging fixture', encoding='utf-8')


def emitted_sources(plan):
    tree = ast.parse(plan['generation']['code'])
    values = {n.targets[0].id: n.value for n in tree.body
              if isinstance(n, ast.Assign) and isinstance(n.targets[0], ast.Name)
              and isinstance(n.value, ast.Constant) and isinstance(n.value.value, str)}
    class Expand(ast.NodeTransformer):
        def visit_Name(self, node):
            return values.get(node.id, node) if isinstance(node.ctx, ast.Load) else node
    writes = literal_writes(ast.unparse(Expand().visit(tree)))
    result = {name.rsplit('/', 1)[-1]: value[0] for name, value in writes.items() if name.endswith('.py')}
    result.update({name.rsplit('/', 1)[-1]: text for name, text in plan['source_packaging']['helpers'].items()})
    return result


class FeatureFlowRecipeTests(unittest.TestCase):
    def test_explicit_seed_recipe_is_complete_and_structured(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root(root)
            plan = build_plan(SELECTION, 'assets/aster/general_face_eyes_combo_cpu_flow/', root,
                              sampling_overrides=FLOW, source_seed=True)
        self.assertIn('source_seed', plan)
        self.assertLessEqual(len(plan['generation']['outputs']), 64)
        self.assertLessEqual(len(plan['generation']['code'].encode()), 1024 * 1024)
        sources = emitted_sources(plan)
        records = plan['source_packaging']['layout']['records']
        ordered = [r for r in records if r['kind'] == 'ordered_module']
        self.assertEqual({r['entry_name'] for r in ordered}, {'face.py', 'shell.py', 'ear_topology.py'})
        restored = {r['entry_name']: reconstructed(sources, r) for r in ordered}
        self.assertIn('capture_oral_skin_donors', restored['face.py'])
        self.assertIn('build_outer_source_flow', restored['shell.py'])
        self.assertIn('def replace_nasal_source(', restored['shell.py'])
        self.assertIn('def regularize_nasal_source(', restored['shell.py'])
        self.assertIn('allocated_flow_records', restored['shell.py'])
        self.assertIn('class ConnectedQuadChart', restored['shell.py'])
        self.assertNotIn('from nasal_source_', restored['shell.py'])
        self.assertIn('shell_nasal_mesh.py', sources)
        self.assertIn('retopologize_flow_ears', restored['ear_topology.py'])
        for forbidden in ('bound_base_ear', 'apply_bound_lower_attachment', 'apply_bound_cheek_strip', 'EAR_RETURN_MINIMAX_EDIT'):
            self.assertNotIn(forbidden, restored['ear_topology.py'])
        for name, source in sources.items():
            compile(source, name, 'exec')
            count = conservative_effective_lines(source)
            self.assertLessEqual(count, 500, name)
        pipeline = ''.join(sources[name] for name in (
            'pipeline_geometry.py', 'pipeline_assembly.py', 'pipeline_runtime.py', 'pipeline_export.py'))
        calls = [n.func.id for n in ast.walk(ast.parse(pipeline)) if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)]
        self.assertNotIn('fit_front_width', calls)
        self.assertEqual(calls.count('restore_oral_skin_donors'), 1)
        self.assertLess(pipeline.index("report['automatic_export_triangle_verification']"),
                        pipeline.index('report["nose_final_attachment_checks"]'))

    def test_unsupported_source_profiles_fail_before_execution(self):
        for name, value in [('sampling_shell_columns', 9), ('sampling_detail_budget', 0),
                            ('sampling_surface_subdivision', 1), ('sampling_surface_fairing', 1),
                            ('sampling_outline_refinement', 1), ('sampling_feature_flow', .5)]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                fixture_root(root)
                with self.assertRaises(ValueError):
                    build_plan(SELECTION, 'assets/aster/general_face_eyes_combo_cpu_invalid/', root,
                               sampling_overrides={**FLOW, name: value}, source_seed=True)

    def test_default_recipe_does_not_emit_ordered_flow_family(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root(root)
            plan = build_plan(SELECTION, 'assets/aster/general_face_eyes_combo_cpu_default/', root)
        self.assertNotIn('source_seed', plan)
        self.assertFalse(any(r['kind'] == 'ordered_module' for r in plan['source_packaging']['layout']['records']))
        self.assertNotIn('capture_oral_skin_donors', plan['generation']['code'])


if __name__ == '__main__':
    unittest.main()
