"""Real recipe packaging and guarded opt-in ear consumers, CPU only."""
import ast
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'face_components'))
sys.path.insert(0, str(ROOT / 'shared_construction_capture'))
from test_feature_flow_recipe import SELECTION, FLOW, fixture_root, emitted_sources
from bundle import build_plan
from source_layout.layout import reconstructed
from source_layout.transport_sources import validate_native_frame
from construction_adapter import POLICY, RUNTIME_MODULES, SOURCE_MODULES, adapt_optional, retained_sources
from construction_ear_adapter import attachment_boundaries
from construction_native import require_current_roles


class EarPackagingTests(unittest.TestCase):
    def test_two_real_profiles_include_bounded_ear_consumers(self):
        for profile in ('research-compact-rounded', 'research-straight-tapered'):
            with self.subTest(profile=profile), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                fixture_root(root)
                plan = build_plan({**SELECTION, 'face': profile}, 'assets/aster/general_face_eyes_combo_ccreview/', root,
                    sampling_overrides=FLOW, construction_identity_capture=POLICY,
                    shared_source_capture="POST_MORPH_SOURCE_CAPTURE_V1", source_seed=True)
                self.assertLessEqual(validate_native_frame(plan['generation']), 1024 * 1024)
                sources = emitted_sources(plan)
                pipeline = sources['pipeline_runtime.py']
                boundaries = ('join_with_ear_capture(members)', 'capture_weld(members[0],',
                    'verify_combined_oral_transfer(', 'verify_ear_membership(', 'prepare_reports(',
                    'fair_lower_ear_boundaries(members[0].data, _construction_ear_reports)',
                    'invalidate_before_surgery(', 'fit_final_ear_support(members[0].data,')
                positions = [pipeline.index(value) for value in boundaries]
                self.assertEqual(positions, sorted(positions))
                self.assertNotIn('join_with_oral_capture(members)', pipeline)
                self.assertIn('"_identity_bridge_triangle", "_identity_root_indices", "_identity_lower"', pipeline)
                for name, symbol in (('ears.py', 'fit_ear_roots'), ('ear_boundary.py', 'fair_lower_ear_boundaries')):
                    records = plan['source_packaging']['layout']['records']
                    record = next((r for r in records if r.get('entry_name') == name), None)
                    source = reconstructed(sources, record) if record else sources[name]
                    node = next(n for n in ast.parse(source).body if isinstance(n, ast.FunctionDef) and n.name == symbol)
                    function = ast.get_source_segment(source, node)
                    if name == 'ears.py':
                        self.assertIn('begin_evaluation(shell)', function)
                        self.assertIn('bind_created_ear(', function)
                    else:
                        self.assertIn('_identity_bridge_triangle', function)
                        self.assertNotIn('KDTree', function)
                for name, source in sources.items():
                    if name.endswith('.py'):
                        compile(source, name, 'exec')

    def test_default_and_retained_inventory(self):
        source = {'unchanged.py': 'sentinel'}
        self.assertIs(adapt_optional(source, None), source)
        runtime = ('ear_contract', 'ear_evaluated', 'ear_join', 'ear_lower_context')
        self.assertTrue(set(runtime) <= set(RUNTIME_MODULES))
        self.assertTrue({'ear_adapter', 'ear_lower_adapter', 'construction_ear_adapter'} <= set(SOURCE_MODULES))
        self.assertEqual(set(retained_sources()), {name + '.py' for name in SOURCE_MODULES})
        for module in runtime:
            source = retained_sources()[module + '.py']
            tree = ast.parse(source)
            for node in tree.body:
                if isinstance(node, ast.ImportFrom) and node.module in RUNTIME_MODULES:
                    self.assertLess(RUNTIME_MODULES.index(node.module), RUNTIME_MODULES.index(module))
        with self.assertRaisesRegex(ValueError, 'not admitted'):
            require_current_roles({'solver_ready': True, 'ear_membership': {}, 'ear_join': {}})

    def test_changed_pipeline_boundary_fails_closed(self):
        with self.assertRaisesRegex(ValueError, 'boundary changed'):
            attachment_boundaries('print("not the original production pipeline")')


if __name__ == '__main__':
    unittest.main()
