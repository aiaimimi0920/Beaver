"""All-child LP survives source retention, flattening and ordered replay."""
import ast
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch
import numpy as np
from scipy.optimize import OptimizeResult
from source_nasal_spacing import adapt
from feature_flow_recipe import retained_sources
from source_layout.module_layout import MODULE_STAGES, module_layout, execute_ordered_module
from test_forehead_complete_guide_stage import fixture
import forehead_seam_restore
import forehead_seam_growth


class ExtremaPackagingTests(unittest.TestCase):
    def flattened(self):
        pipeline = '''def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
'''
        return adapt({'pipeline.py': pipeline, 'shell.py': ''})['shell.py']

    def verify_model_and_LP(self, namespace):
        p, proposal, config = fixture()
        config['max_evaluations'] = 10000
        stage = namespace['ForeheadCompleteGuideFairingStage'](proposal, p, 'packaged', config)
        differential = np.zeros((len(p), 3, 2))
        differential[:, 0, 0] = differential[:, 2, 1] = 1
        c = stage.constraints(p)
        model = namespace['forehead_role_extrema_model'](stage, p, np.arange(len(p)), p,
            differential, np.zeros(len(p), bool), 0, np.ones(stage.count), c)
        self.assertGreater(len(model.owners), 30)
        rows = namespace['forehead_extrema_rows'](c, np.zeros((stage.count, 2*len(p))), model)
        self.assertEqual(len(rows[0]), stage.count-len(model.handled_rows)+len(model.owners))
        self.assertTrue(callable(namespace['forehead_make_extrema_model']))
        cls = namespace['ForeheadExtremaLinearModel']
        from scipy.sparse import csr_matrix
        result, metadata = namespace['forehead_restore_linearized'](
            np.array([0.]), np.array([1.]), lambda x: np.array([x[0]-.5, 1.]),
            lambda x: csr_matrix([[1.], [0.]]), lambda x: x[0] >= .5, (1, 2), lambda x: True,
            linear_model=lambda x: cls(np.array([0, 0]), np.array([x[0]-.5, 1.]),
                csr_matrix([[1.], [0.]]), (0,), (), (), 2), max_iterations=1, wall_seconds=2.)
        self.assertTrue(result.success)
        self.assertFalse(metadata['full_model_success'])

    def test_retention_and_flattened_runtime(self):
        sources = retained_sources()
        for name in ('primitives', 'model', 'lp'):
            self.assertIn('forehead_extrema_'+name+'.py', sources)
        namespace = {}
        exec(compile(self.flattened(), 'flattened-extrema-test', 'exec'), namespace)
        self.verify_model_and_LP(namespace)

    def test_real_ordered_module_sections_replay(self):
        flat = self.flattened()
        present = {node.name for node in ast.parse(flat).body
                   if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        # Source_nasal_spacing appends to the already-built shell. Supply only
        # missing earlier stage markers, then replay its complete actual payload.
        earlier = 'pass\n'+''.join('def '+name+'():\n    pass\n'
            for _, name in MODULE_STAGES['shell.py'][1:] if name not in present)
        files, record = module_layout(earlier+flat, 'shell.py')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, text in files.items():
                (root/name).write_text(text)
            module = types.ModuleType('extrema_packaged_shell')
            execute_ordered_module(root/'shell.py', record, module.__dict__)
            self.verify_model_and_LP(module.__dict__)
        self.assertIn('shell_forehead_extrema_model.py', files)
        self.assertIn('shell_forehead_extrema_lp.py', files)

    def test_same_callback_forwarded_to_both_legacy_legs(self):
        callback = lambda x: None
        calls = []
        clock = [0.]
        def solve(*args, **kwargs):
            calls.append(kwargs)
            clock[0] += 60.
            return OptimizeResult(x=np.zeros(1), nit=0, success=False, message='test'), {'steps': []}
        with patch.object(forehead_seam_restore, 'forehead_restore_linearized', side_effect=solve), \
                patch.object(forehead_seam_restore.time, 'monotonic', side_effect=lambda: clock[0]):
            forehead_seam_restore.forehead_restore_seam_stage(np.zeros(1), np.ones(1),
                None, None, None, (0, 0), None, {'held_columns': [0]}, None, np.ones(0),
                linear_model=callback)
        self.assertEqual(len(calls), 2)
        self.assertTrue(all(row['linear_model'] is callback for row in calls))
        self.assertEqual([row['temporary_held_columns'] for row in calls], [[0], []])
        self.assertLessEqual(sum(row['wall_seconds'] for row in calls), 120.001)

    def test_growth_forwards_callback_without_changing_budget(self):
        callback = lambda x: None
        with patch.object(forehead_seam_growth, 'forehead_restore_seam_stage',
                          return_value=('original', 'metadata')) as legacy:
            result = forehead_seam_growth.forehead_restore_seam_growth(np.zeros(1), np.ones(1),
                None, None, None, (0, 0), None, {}, None, None, linear_model=callback)
        self.assertEqual(result, ('original', 'metadata'))
        self.assertIs(legacy.call_args.kwargs['linear_model'], callback)
        calls = []
        def solve(*args, **kwargs):
            calls.append(kwargs)
            return OptimizeResult(x=np.zeros(1), nit=0, success=False, message='test'), {'steps': []}
        with patch.object(forehead_seam_growth, 'forehead_restore_linearized', side_effect=solve):
            forehead_seam_growth.forehead_restore_seam_growth(np.zeros(1), np.ones(1),
                None, None, None, (0, 0), None, {}, None, None, linear_model=callback,
                active_set_growth=True, growth_context={})
        self.assertEqual(len(calls), 1)
        self.assertIs(calls[0]['linear_model'], callback)
        self.assertLessEqual(calls[0]['wall_seconds'], 120.)


if __name__ == '__main__':
    unittest.main()
