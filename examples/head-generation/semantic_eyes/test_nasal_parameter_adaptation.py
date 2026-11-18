"""Bounded failure handling and nonmutation for current-surface adaptation."""
import unittest
from unittest.mock import patch
import numpy as np
from nasal_parameter_adaptation import adapt_nasal_patch_parameters
from nasal_source_binding import validate_patch
from nasal_source_geometry import AuthorQuadSurface


class NasalParameterAdaptationTests(unittest.TestCase):
    def setUp(self):
        self.boundary = np.array([(0.,0.,0.),(.02,0.,0.),(.02,0.,.02),(0.,0.,.02)])
        self.surface = AuthorQuadSurface(self.boundary, [(0,1,2,3)], [1])
        self.points = np.empty((0,3))
        self.lineage = []

    def test_valid_input_is_exact_identity_without_optimizer(self):
        with patch('nasal_parameter_adaptation.least_squares', side_effect=AssertionError('must not optimize')):
            points, lineage, report = adapt_nasal_patch_parameters(
                self.surface, self.boundary, self.points, [(0,1,2,3)], self.lineage, validate_patch)
        self.assertIs(points, self.points)
        self.assertIs(lineage, self.lineage)
        self.assertEqual(report['status'], 'unchanged_original_gate_passed')
        self.assertEqual(report['quality']['samples'], 28)

    def test_fold_is_not_retried(self):
        with patch('nasal_parameter_adaptation.least_squares', side_effect=AssertionError('must not optimize')):
            with self.assertRaisesRegex(ValueError, 'folded'):
                adapt_nasal_patch_parameters(self.surface, self.boundary, self.points,
                                            [(0,3,2,1)], self.lineage, validate_patch)

    def test_nonfinite_input_rejected_before_validation(self):
        self.boundary[0,0] = np.nan
        with self.assertRaisesRegex(ValueError,'Nonfinite'):
            adapt_nasal_patch_parameters(self.surface,self.boundary,self.points,
                                        [(0,1,2,3)],self.lineage,validate_patch)

    def test_fixed_boundary_failure_does_not_move_boundary(self):
        displaced = self.boundary.copy()
        displaced[:,1] = -.002
        before = displaced.copy()
        with self.assertRaisesRegex(ValueError, 'variable profile'):
            adapt_nasal_patch_parameters(self.surface, displaced, self.points,
                                        [(0,1,2,3)], self.lineage, validate_patch)
        np.testing.assert_array_equal(displaced, before)


if __name__ == '__main__':
    unittest.main()
