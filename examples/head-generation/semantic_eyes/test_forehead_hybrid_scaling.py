import unittest
import numpy as np
from scipy.sparse import csr_matrix
from forehead_hybrid_scaling import forehead_prepare_hybrid_scaling


class ForeheadHybridScalingTests(unittest.TestCase):
    def test_fixed_norms_cached_matrix_and_later_closure(self):
        start = np.array([1., 2.])
        scales = np.ones(3)
        calls = []
        matrix = np.array([[3., 4.], [0., .5], [0., 0.]])
        def derivative(x):
            calls.append(x.copy())
            return csr_matrix((matrix*x[0])/scales[:, None])
        bound, evidence = forehead_prepare_hybrid_scaling(derivative, start, scales)
        np.testing.assert_array_equal(scales, [5., 1., 1.])
        self.assertFalse(scales.flags.writeable)
        first = bound(start)
        np.testing.assert_allclose(first.toarray(), matrix/scales[:, None])
        first.data[:] = 999.
        np.testing.assert_allclose(bound(start).toarray(), matrix/scales[:, None])
        self.assertEqual(len(calls), 1)
        np.testing.assert_allclose(bound(np.array([2., 2.])).toarray(), 2*matrix/scales[:, None])
        self.assertEqual(len(calls), 2)
        self.assertEqual(evidence['initial_cache_hits'], 2)

    def test_raw_feasible_set_signs_are_unchanged(self):
        scales = np.ones(3)
        forehead_prepare_hybrid_scaling(lambda x: csr_matrix([[30., 40.], [0., 2.], [0., 0.]]), np.ones(2), scales)
        raw = np.array([-1e-9, 0., 7.])
        np.testing.assert_array_equal(raw >= 0, raw/scales >= 0)
        np.testing.assert_allclose(raw/scales*scales, raw, rtol=0., atol=1e-20)

    def test_invalid_matrix_rejected_before_scales_change(self):
        for matrix in (np.zeros((2, 2)), np.array([[np.nan, 1.]])):
            scales = np.ones(1)
            with self.assertRaises(ValueError):
                forehead_prepare_hybrid_scaling(lambda x: matrix, np.ones(2), scales)
            np.testing.assert_array_equal(scales, [1.])
            self.assertTrue(scales.flags.writeable)

    def test_scaled_or_invalid_inputs_are_not_reused(self):
        for scales in (np.array([2.]), np.array([1]), np.array([np.nan])):
            with self.assertRaises(ValueError):
                forehead_prepare_hybrid_scaling(lambda x: np.ones((1, 2)), np.ones(2), scales)
        scales = np.ones(1)
        scales.setflags(write=False)
        with self.assertRaises(ValueError):
            forehead_prepare_hybrid_scaling(lambda x: np.ones((1, 2)), np.ones(2), scales)


if __name__ == '__main__':
    unittest.main()
