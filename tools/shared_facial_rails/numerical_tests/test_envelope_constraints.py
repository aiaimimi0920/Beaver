"""Synthetic numerical regressions; no character assets or native adapters."""
import unittest
import numpy as np
from shared_facial_rails.envelope_constraints import edge_length_envelope_linearization as envelope

class Envelopes(unittest.TestCase):

    def setUp(self):
        r = np.random.default_rng(795)
        self.p = r.normal(size=(8, 3))
        self.w = r.normal(size=(8, 3, 4)) * 0.1
        self.e = np.array([[0, 1], [2, 3], [4, 5], [6, 7]])

    def make(self, p=None, saved=None, **kwargs):
        p = self.p if p is None else p
        return envelope(p if saved is None else saved, p, self.w, self.e, 2.0, 0.4, **kwargs)

    def test_finite_difference_and_full_mean_derivative(self):
        z = self.make()
        h = 1e-06
        fd = np.stack([(self.make(self.p + h * self.w[:, :, k])['slack'] - self.make(self.p - h * self.w[:, :, k])['slack']) / (2 * h) for k in range(4)], 1)
        np.testing.assert_allclose(z['jacobian'], fd, atol=1e-08, rtol=1e-07)
        p = np.array([[0.0, 0, 0], [1.0, 0, 0], [0.0, 2, 0], [2.0, 2, 0]])
        w = np.zeros((4, 3, 1))
        w[3, 0, 0] = 1
        z = envelope(p, p, w, [[0, 1], [2, 3]], 2.0, 1.0)
        i = z['labels'].index(('relative_upper', 0))
        self.assertAlmostEqual(z['jacobian'][i, 0], 2 / 9)

    def test_exact_envelopes(self):
        r = np.random.default_rng(97)
        for n in range(2, 12):
            for _ in range(25):
                l = np.exp(r.normal(size=n))
                p = np.zeros((2 * n, 3))
                p[1::2, 0] = l
                e = np.arange(2 * n).reshape(n, 2)
                R = r.uniform(1, 6)
                E = r.uniform(0, 2)
                z = envelope(p, p, np.zeros((2 * n, 3, 1)), e, R, E)
                ratio = [s for s, t in zip(z['slack'], z['labels']) if t[0] == 'ratio']
                relative = [s for s, t in zip(z['slack'], z['labels']) if t[0] != 'ratio']
                self.assertEqual(min(ratio) >= 0, l.max() / l.min() <= R)
                self.assertEqual(min(relative) >= 0, np.max(abs(l / l.mean() - 1)) <= E)
                self.assertEqual(len(z['slack']), n * (n - 1) + 2 * n)

    def test_branch_switch(self):
        p = np.zeros((6, 3))
        p[1::2, 0] = [1, 1.01, 1.5]
        w = np.zeros((6, 3, 1))
        w[1, 0, 0] = 0.02
        w[3, 0, 0] = -0.04
        z = envelope(p, p, w, [[0, 1], [2, 3], [4, 5]], 1.5, 1.0)
        i = z['labels'].index(('ratio', 2, 1))
        old_minimum = z["labels"].index(("ratio", 2, 0))
        self.assertGreaterEqual(z["slack"][old_minimum] +
                                z["jacobian"][old_minimum, 0], 0)
        self.assertGreater(1.5 / 0.97, 1.5)
        self.assertLess(z['slack'][i] + z['jacobian'][i, 0], 0)

    def test_reduced_activity_preserves_negative_held_slack(self):
        p = np.zeros((4, 3))
        p[1, 0] = p[3, 0] = 1
        w = np.zeros((4, 3, 2))
        w[1, 0, 1] = 1
        z = envelope(p, p, w, [[0, 1], [2, 3]], 1.0, 0.0, active_columns=[0])
        self.assertTrue(np.all(z['slack'] == 0))
        self.assertTrue(np.all(z['reserve'] == 0))
        zfull = envelope(p, p, w, [[0, 1], [2, 3]], 1.0, 0.0)
        self.assertTrue(np.all(zfull['reserve'] > 0))
        p[3, 0] = 1.1
        z = envelope(p, p, w, [[0, 1], [2, 3]], 1.0, 0.0, active_columns=[])
        self.assertLess(z['rhs'].min(), 0)
        np.testing.assert_array_equal(z['rhs'], z['slack'])

    def test_saved_exact_rounding_reserve(self):
        saved = self.p.astype(np.float32).astype(float)
        z = self.make(saved=saved)
        np.testing.assert_array_equal(z['jacobian'], self.make()['jacobian'])
        np.testing.assert_array_equal(z['slack'], self.make(saved)['slack'])
        np.testing.assert_allclose(z['reserve'], np.maximum(8 * abs(z['slack'] - z['exact_slack']) + 1e-10, 1e-07))
        z = self.make(saved=saved, reserve_floor=2e-06)
        self.assertTrue(np.all(z['reserve'] >= 2e-06))

    def test_translation_and_validation(self):
        w = np.broadcast_to(np.eye(3), (8, 3, 3)).copy()
        z = envelope(self.p, self.p, w, self.e, 2.0, 0.4)
        self.assertTrue(np.all(z['jacobian'] == 0))
        self.assertTrue(np.all(z['reserve'] == 0))
        for edges in ([], [[0, 1]], [[0, 1], [2, 100]], [[0.0, 1.0], [2.0, 3.0]]):
            with self.assertRaises(ValueError):
                envelope(self.p, self.p, self.w, edges, 2.0, 0.4)
        p = self.p.copy()
        p[1] = p[0]
        with self.assertRaises(ValueError):
            self.make(p)
        with self.assertRaises(ValueError):
            self.make(active_columns=[99])

class ValidationAndInvariance(unittest.TestCase):

    def setUp(self):
        self.p = np.zeros((6, 3))
        self.p[1::2, 0] = [1.0, 1.1, 1.4]
        self.w = np.random.default_rng(21).normal(size=(6, 3, 2))
        self.e = np.arange(6).reshape(3, 2)

    def make(self, **changes):
        args = dict(p_saved=self.p, p_exact=self.p, world_J=self.w, edges=self.e, ratio_cap=1.5, relative_cap=0.5)
        args.update(changes)
        return envelope(**args)

    def test_vertex_edge_and_parameter_permutations(self):
        baseline = self.make()
        vertices = np.array([4, 1, 5, 3, 0, 2])
        inverse = np.argsort(vertices)
        order = np.array([2, 0, 1])
        result = self.make(p_saved=self.p[vertices], p_exact=self.p[vertices], world_J=self.w[vertices, :, ::-1], edges=inverse[self.e[order, ::-1]])
        for index, label in enumerate(result['labels']):
            old = (label[0], *(int(order[i]) for i in label[1:]))
            source = baseline['labels'].index(old)
            for key in ('slack', 'exact_slack', 'rhs', 'reserve', 'active'):
                np.testing.assert_allclose(result[key][index], baseline[key][source])
            np.testing.assert_allclose(result['jacobian'][index], baseline['jacobian'][source, ::-1])

    def test_stationary_derivative_is_not_structural_invariance(self):
        p = np.zeros((4, 3))
        p[1, 0] = p[3, 0] = 1
        w = np.zeros((4, 3, 1))
        w[1, 1, 0] = 1
        result = envelope(p, p, w, [[0, 1], [2, 3]], 1.0, 0.0)
        self.assertFalse(result['active'].any())
        self.assertTrue(np.all(result['rhs'] == 0))
        moved = p + 0.1 * w[:, :, 0]
        nonlinear = envelope(moved, moved, w, [[0, 1], [2, 3]], 1.0, 0.0)
        self.assertLess(nonlinear['slack'].min(), 0)

    def test_nonfinite_shapes_caps_and_indices(self):
        cases = [dict(p_saved=np.zeros((6, 2))), dict(p_exact=np.zeros((5, 3))), dict(world_J=np.zeros((6, 3))), dict(world_J=np.zeros((6, 3, 0))), dict(edges=[[0, 1], [-1, 2]]), dict(edges=[[0, 0], [2, 3]]), dict(edges=np.array([[0, 1], [2, 2 ** 64 - 1]], dtype=np.uint64)), dict(active_columns=[[0]]), dict(active_columns=[0.5]), dict(active_columns=[True]), dict(active_columns=[2])]
        for name, values in {'ratio_cap': [0, np.inf, np.nan, [2], True], 'relative_cap': [-1, np.inf, np.nan, [1]], 'reserve_floor': [-1, np.inf, [0]], 'activity_tolerance': [-1, np.nan, [0]]}.items():
            cases.extend(({name: value} for value in values))
        for name in ('p_saved', 'p_exact', 'world_J'):
            original = self.w if name == 'world_J' else self.p
            for value in (np.nan, np.inf, -np.inf):
                array = original.copy()
                array.flat[0] = value
                cases.append({name: array})
        for changes in cases:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                self.make(**changes)

    def test_derived_overflow_and_underflow_fail_closed(self):
        for scale in (1e+308, 1e-300):
            with self.subTest(scale=scale), self.assertRaises(ValueError):
                self.make(p_saved=self.p * scale, p_exact=self.p * scale)
        w = np.full_like(self.w, 1e+308)
        w[::2] = -1e+308
        with self.assertRaises(ValueError):
            self.make(world_J=w)
        saved = self.p.copy()
        saved[1, 0] = 1e-154
        saved[3, 0] = 1e+154
        with self.assertRaises(ValueError):
            self.make(p_saved=saved)

    def test_empty_and_selected_columns_do_not_remove_rows(self):
        baseline = self.make()
        held = self.make(active_columns=[])
        selected = self.make(active_columns=[1])
        self.assertEqual(held['labels'], baseline['labels'])
        np.testing.assert_array_equal(held['jacobian'], baseline['jacobian'])
        np.testing.assert_array_equal(selected['jacobian'], baseline['jacobian'])
        self.assertFalse(held['active'].any())
        self.assertTrue(np.all(held['reserve'] == 0))

    def test_rejects_nonreal_geometry_before_conversion(self):
        for name in ('p_saved', 'p_exact', 'world_J'):
            original = self.w if name == 'world_J' else self.p
            cases = [original.astype(kind) for kind in (complex, object, str, bool)]
            for imaginary in (1., float('nan'), float('inf')):
                array = original.astype(complex)
                array.flat[0] = complex(0., imaginary)
                cases.append(array)
            for array in cases:
                with self.subTest(name=name, dtype=array.dtype), self.assertRaises(ValueError):
                    self.make(**{name: array})

    def test_accepts_real_numeric_arrays_and_rejects_cast_overflow(self):
        for dtype in (np.int32, np.uint64, np.float32, np.float64):
            p = self.p.astype(dtype)
            w = np.zeros(self.w.shape, dtype=dtype)
            result = self.make(p_saved=p, p_exact=p, world_J=w)
            self.assertTrue(np.isfinite(result['jacobian']).all())
            self.assertTrue(np.isfinite(result['slack']).all())
        if np.finfo(np.longdouble).max > np.finfo(float).max:
            for name in ('p_saved', 'p_exact', 'world_J'):
                original = self.w if name == 'world_J' else self.p
                array = original.astype(np.longdouble)
                array.flat[0] = np.finfo(np.longdouble).max
                with self.subTest(name=name), self.assertRaises(ValueError):
                    self.make(**{name: array})
