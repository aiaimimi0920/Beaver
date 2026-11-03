import unittest
import numpy as np
from scipy.sparse import csr_matrix
from forehead_exact_jacobian_cache import forehead_exact_fit_cache as exact_fit_cache, forehead_exact_epoch_digest as epoch_digest

class CacheTests(unittest.TestCase):

    def setUp(self):
        self.state = {'source': np.array([[1.0, 2.0]]), 'scales': np.array([1.0, 2.0]), 'caps': [0.1, 0.5], 'source_hash': 'immutable-source', 'implementation_hash': 'code-a'}
        self.calls = []
        self.charges = []

        def derivative(x):
            self.calls.append(x.copy())
            self.charges.append('assembly')
            return csr_matrix([[x[0] + 1, x[1]], [x[1], x[0] + 2]])
        self.derivative = derivative
        self.f, self.m = exact_fit_cache(derivative, lambda: self.state, self.charges.append, 2, 2)

    def test_hit_and_request_count(self):
        a = self.f([1, 2])
        b = self.f([1, 2])
        np.testing.assert_array_equal(a.toarray(), b.toarray())
        self.assertEqual(len(self.calls), 1)
        self.assertEqual(self.charges.count('exact_fit_jacobian_request'), 2)

    def test_one_ulp_is_not_same_candidate(self):
        self.f([1, 2])
        self.f([np.nextafter(1.0, 2.0), 2])
        self.assertEqual(len(self.calls), 2)

    def test_signed_zero_is_not_same_bytes(self):
        self.f([0.0, 2])
        self.f([-0.0, 2])
        self.assertEqual(len(self.calls), 2)

    def test_lru_bounded(self):
        for x in ([1, 2], [2, 2], [1, 2], [3, 2], [2, 2]):
            self.f(x)
        self.assertEqual(len(self.calls), 4)
        self.assertEqual(self.m['hits'], 1)

    def test_defensive_sparse_copy(self):
        a = self.f([1, 2])
        a.data[:] = 999
        a.indices[:] = 0
        np.testing.assert_array_equal(self.f([1, 2]).toarray(), [[2, 2], [2, 3]])

    def test_input_not_mutated(self):
        x = np.array([1.0, 2.0])
        self.f(x)
        np.testing.assert_array_equal(x, [1, 2])
        x[0] = 3
        self.f(x)
        self.assertEqual(len(self.calls), 2)

    def test_every_dependency_invalidates(self):
        for key, value in [('source', np.array([[3.0, 2.0]])), ('scales', np.array([2.0, 2.0])), ('caps', [0.2, 0.5]), ('source_hash', 'other'), ('implementation_hash', 'code-b')]:
            self.setUp()
            self.f([1, 2])
            self.state[key] = value
            with self.assertRaisesRegex(ValueError, 'epoch changed'):
                self.f([1, 2])
            self.assertEqual(len(self.calls), 1)
            self.assertEqual(self.m['requests'], 2)

    def test_in_place_mutation_invalidates(self):
        self.f([1, 2])
        self.state['source'][0, 0] = 4
        with self.assertRaises(ValueError):
            self.f([1, 2])

    def test_invalidated_remains_poisoned(self):
        self.f([1, 2])
        self.state['caps'] = [2, 3]
        with self.assertRaises(ValueError):
            self.f([1, 2])
        self.state['caps'] = [0.1, 0.5]
        with self.assertRaises(ValueError):
            self.f([1, 2])

    def test_mutating_derivative_fails(self):

        def bad(x):
            self.state['caps'] = [1, 2]
            return csr_matrix(np.eye(2))
        f, _ = exact_fit_cache(bad, lambda: self.state, self.charges.append, 2, 2)
        with self.assertRaises(ValueError):
            f([1, 2])

    def test_budget_rejects_even_cache_hit(self):
        count = [0]

        def charge(k):
            if count[0] >= 1:
                raise ValueError('budget')
            count[0] += 1
        f, _ = exact_fit_cache(self.derivative, lambda: self.state, charge, 2, 2)
        f([1, 2])
        with self.assertRaisesRegex(ValueError, 'budget'):
            f([1, 2])
        self.assertEqual(len(self.calls), 1)

    def test_bad_inputs_are_charged(self):
        for x in ([1], [float('nan'), 2], [[1, 2]]):
            with self.assertRaises(ValueError):
                self.f(x)
        self.assertEqual(len(self.calls), 0)
        self.assertEqual(self.m['requests'], 3)

    def test_invalid_matrix_never_cached(self):
        for matrix in (np.eye(3), np.array([[float('nan'), 1], [1, 2]])):
            f, m = exact_fit_cache(lambda x: csr_matrix(matrix), lambda: self.state, self.charges.append, 2, 2)
            with self.assertRaises(ValueError):
                f([1, 2])
            with self.assertRaises(ValueError):
                f([1, 2])
            self.assertEqual(m['hits'], 0)

    def test_separate_fits_never_share_cache(self):
        self.f([1, 2])
        f, _ = exact_fit_cache(self.derivative, lambda: self.state, self.charges.append, 2, 2)
        f([1, 2])
        self.assertEqual(len(self.calls), 2)

    def test_failed_derivative_not_cached(self):

        def fail(x):
            raise RuntimeError('failure')
        f, m = exact_fit_cache(fail, lambda: self.state, self.charges.append, 2, 2)
        for i in range(2):
            with self.assertRaises((RuntimeError, ValueError)):
                f([1, 2])
        self.assertEqual(m['hits'], 0)

    def test_unserializable_epoch_poisons(self):
        self.f([1, 2])
        self.state['bad'] = object()
        with self.assertRaises(ValueError):
            self.f([1, 2])
        del self.state['bad']
        with self.assertRaises(ValueError):
            self.f([1, 2])

    def test_structured_dtype_rejected(self):
        with self.assertRaises(ValueError):
            epoch_digest(np.zeros(1, dtype=[('a', 'f8')]))

    def test_failed_assembly_poison_prevents_old_hits(self):
        count = [0]

        def once(x):
            count[0] += 1
            if count[0] > 1:
                raise RuntimeError('failed')
            return csr_matrix(np.eye(2))
        f, m = exact_fit_cache(once, lambda: self.state, self.charges.append, 2, 2)
        f([1, 2])
        with self.assertRaises(RuntimeError):
            f([2, 2])
        with self.assertRaises(ValueError):
            f([1, 2])
        self.assertTrue(m['invalidated'])

    def test_capacity_strictly_two(self):
        for n in (0, 1, 3, 4, True):
            with self.assertRaises(ValueError):
                exact_fit_cache(self.derivative, lambda: self.state, self.charges.append, 2, 2, n)

    def test_digest_dtype_shape_values(self):
        a = np.array([1.0, 2.0])
        base = epoch_digest(a)
        for b in (a.astype('f4'), a.reshape(1, 2), np.array([1.0, 3.0])):
            self.assertNotEqual(base, epoch_digest(b))
if __name__ == '__main__':
    unittest.main()
