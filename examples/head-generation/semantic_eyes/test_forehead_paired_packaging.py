"""Paired selection and guarded fallback survive both generated module forms."""
import ast
import builtins
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import numpy as np
from scipy.optimize._numdiff import approx_derivative

from feature_flow_recipe import retained_sources
from forehead_surface_batch import ForeheadSurfaceBatch
from local_shape_safe_derivative import local_shape_safe_derivative
from source_nasal_spacing import adapt
from target_boundary_dependency import target_has_boundary_dependency
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.layout import reconstructed
from source_layout.module_layout import MODULE_STAGES, execute_ordered_module, module_layout


HELPERS = {
    'target_boundary_dependency': 'target_has_boundary_dependency',
    'local_shape_safe_derivative': 'local_shape_safe_derivative',
}


def guarded_import(name, *args, **kwargs):
    if name in HELPERS:
        raise AssertionError('Generated source leaked helper import: ' + name)
    return ORIGINAL_IMPORT(name, *args, **kwargs)


ORIGINAL_IMPORT = builtins.__import__


def flattened_source():
    pipeline = '''def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
'''
    return adapt({'pipeline.py': pipeline, 'shell.py': ''})['shell.py']


class PairedPackagingTests(unittest.TestCase):
    def check_runtime(self, namespace):
        chart = np.array([[0., 0., 0.], [1., 0., 0.],
                          [1., 0., 1.], [0., 0., 1.]])
        vertices = chart.copy()
        vertices[:, 1] = [0., .1, .3, .2]
        quads = np.array([[0, 1, 2, 3]])
        query = np.array([[.2, .3], [0., .5], [1., 1.]])
        expected = ForeheadSurfaceBatch(chart, vertices, quads).sample_differential(query)
        actual = namespace['ForeheadSurfaceBatch'](chart, vertices, quads).sample_differential(query)
        for old, new in zip(expected, actual):
            np.testing.assert_array_equal(old, new)
        args = ([0, 1], [2**-31], [0], {1: 0}, np.array([True]))
        self.assertTrue(target_has_boundary_dependency(*args))
        self.assertEqual(namespace['target_has_boundary_dependency'](*args),
                         target_has_boundary_dependency(*args))
        # Missing whole differentials use the same guarded scalar function.
        values = np.array([2., 3.])
        evaluate = lambda x: (np.array([[x[0], 0., x[1]]]), np.zeros((1, 3)))
        constraints = lambda x: np.array([x[0] + 2*x[1], 3*x[0] - x[1]])
        batch = SimpleNamespace(sample_differential=lambda x: None)
        empty = SimpleNamespace(count=0)
        args = (values, evaluate, constraints, constraints, batch, batch,
                [0], empty, empty, 2, approx_derivative)
        expected = local_shape_safe_derivative(*args)
        actual = namespace['local_shape_safe_derivative'](*args)
        np.testing.assert_array_equal(actual, expected)
        np.testing.assert_allclose(actual, [[1., 2.], [3., -1.]], rtol=0, atol=1e-9)
        for consumer in ('forehead_vertex_gap_row', 'forehead_matched_gap_jacobian'):
            self.assertIs(namespace[consumer].__globals__['target_has_boundary_dependency'],
                          namespace['target_has_boundary_dependency'])
        self.assertIs(namespace['forehead_condition_local_shape'].__globals__[
            'local_shape_safe_derivative'], namespace['local_shape_safe_derivative'])

    def test_retained_sources_are_exact_and_unique(self):
        from feature_flow_recipe import DEPENDENCIES
        from source_nasal_spacing import SPACING_MODULES
        sources = retained_sources()
        root = Path(__file__).parent
        for module in HELPERS:
            self.assertEqual(DEPENDENCIES.count(module), 1)
            self.assertEqual(SPACING_MODULES.count(module), 1)
            self.assertEqual(sources[module + '.py'], (root / (module + '.py')).read_text())

    def test_flattened_helpers_bind_without_local_imports(self):
        source = flattened_source()
        tree = ast.parse(source)
        for module, function in HELPERS.items():
            self.assertFalse(any(isinstance(node, ast.ImportFrom) and node.module == module
                                 for node in ast.walk(tree)))
            self.assertEqual(sum(isinstance(node, ast.FunctionDef) and node.name == function
                                 for node in tree.body), 1)
        namespace = {}
        with patch('builtins.__import__', side_effect=guarded_import):
            exec(compile(source, 'paired-flat', 'exec'), namespace)
            self.check_runtime(namespace)

    def test_ordered_modules_reconstruct_and_execute_the_same_helpers(self):
        source = flattened_source()
        present = {node.name for node in ast.parse(source).body
                   if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        earlier = 'pass\n' + ''.join('def ' + name + '():\n    pass\n'
            for _, name in MODULE_STAGES['shell.py'][1:] if name not in present)
        files, record = module_layout(earlier + source, 'shell.py')
        self.assertEqual(reconstructed(files, record), earlier + source)
        for module in HELPERS:
            self.assertIn('shell_' + module + '.py', files)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, text in files.items():
                (root / name).write_text(text)
            namespace = {}
            with patch('builtins.__import__', side_effect=guarded_import):
                execute_ordered_module(root / 'shell.py', record, namespace)
                self.check_runtime(namespace)


if __name__ == '__main__':
    unittest.main()
