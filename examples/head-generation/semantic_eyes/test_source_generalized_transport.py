import copy
import unittest
from unittest.mock import patch

import numpy as np

from source_generalized_transport import (
    generalized_corner_values, generalized_fixed_quad_triangles,
    generalized_fixed_triangle_poses, generalized_source_binding,
    generalized_source_values, generalized_source_weights,
    generalized_sparse_readset,
)


class GeneralizedTransportTests(unittest.TestCase):
    def setUp(self):
        self.rows = [[i, [1, 4]] for i in range(4)]
        self.hashes = {k: str(i) * 64 for i, k in enumerate(
            ('raw', 'author', 'frozen_fields'))}
        self.binding = generalized_source_binding(
            self.rows, source_hashes=self.hashes, frozen_field_identity='old:17')
        self.xyz = np.array([[0., 0., 0.], [1., 0., 0.],
                             [1., 1., 1.], [0., 1., 0.]])
        self.triangles = [[0, 1, 2], [0, 2, 3]]

    def test_exact_rational_no_pruning(self):
        rows = [[0, [1, 10**20]], [1, [10**20-1, 10**20]]]
        ids, weights = generalized_source_weights(rows)
        self.assertEqual(ids, [0, 1])
        self.assertGreater(weights[0], 0)
        with self.assertRaises(ValueError):
            generalized_source_values(rows, np.array([0, 1]), categorical=True)

    def test_bad_weights(self):
        for rows in ([], [[0, [1, 2]]], [[0, [-1, 2]], [1, [3, 2]]],
                     [[0, [1, 0]]], [[True, [1, 1]]],
                     [[0, [1, 2]], [0, [1, 2]]], [[0, [1., 1]]]):
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                generalized_source_weights(rows)

    def test_generalized_not_triangle_or_bilinear(self):
        rows = [[0, [1, 8]], [1, [1, 8]], [2, [1, 4]], [3, [1, 2]]]
        value = generalized_source_values(rows, self.xyz)
        np.testing.assert_array_equal(value, [0.375, 0.75, 0.25])
        # Bilinear(s=.375,t=.75) has z=.28125: never silently use that API.
        self.assertNotEqual(value[2], value[0] * value[1])
        self.assertTrue(self.binding['not_single_triangle_or_bilinear_binding'])

    def test_binding_copies_and_requires_baselines(self):
        self.rows[0][1][0] = 99
        self.hashes['raw'] = 'changed'
        self.assertEqual(self.binding['source_weights_rational'][0][1], [1, 4])
        with self.assertRaises(ValueError):
            generalized_source_binding([[0, [1, 1]]], source_hashes={},
                                       frozen_field_identity='old:17')

    def test_categories_and_float_rounding_separate(self):
        np.testing.assert_equal(generalized_source_values(
            self.rows, np.ones(4, dtype=int), categorical=True), 1)
        with self.assertRaises(ValueError):
            generalized_source_values(self.rows, np.arange(4), categorical=True)
        with self.assertRaises(ValueError):
            generalized_source_values(self.rows, np.arange(4))
        out = generalized_source_values(self.rows, self.xyz.astype(np.float32))
        self.assertEqual(out.dtype, np.float64)

    def test_corner_owner_no_seam_average(self):
        kwargs = dict(owner=4, layer='uv', loop_vertices=[0, 1, 2, 3],
                      loop_owners=[4]*4, loop_values=self.xyz[:, :2],
                      loop_ids=[0, 1, 2, 3])
        np.testing.assert_array_equal(generalized_corner_values(self.rows, **kwargs),
                                      [0.5, 0.5])
        kwargs['loop_owners'] = [4, 4, 5, 4]
        with self.assertRaises(ValueError):
            generalized_corner_values(self.rows, **kwargs)

    def test_sparse_stale_rebase_and_birth_collision(self):
        actual = {'old': self.binding, 'unrelated': {'untouched': 7}}
        expected = {'old': copy.deepcopy(self.binding)}
        operations = [dict(operation='replace', identity='old',
                           record=copy.deepcopy(self.binding))]
        before = copy.deepcopy(actual)
        self.assertEqual(len(generalized_sparse_readset(expected, actual, operations)), 64)
        self.assertEqual(actual, before)
        operations[0]['record']['frozen_field_identity'] = 'reset'
        with self.assertRaises(ValueError):
            generalized_sparse_readset(expected, actual, operations)
        with self.assertRaises(ValueError):
            generalized_sparse_readset(expected, actual,
                                       [dict(operation='append', identity='old')])
        expected['old']['source_hashes']['author'] = 'f'*64
        with self.assertRaises(ValueError):
            generalized_sparse_readset(expected, actual, [])

    def test_exact_partition_both_diagonals_and_reindexed(self):
        for tris in (self.triangles, [[1, 2, 3], [1, 3, 0]]):
            self.assertTrue(generalized_fixed_quad_triangles([[0, 1, 2, 3]], tris, [0, 0]))
        self.assertTrue(generalized_fixed_quad_triangles(
            [[9, 3, 11, 7]], [[9, 3, 11], [9, 11, 7]], [0, 0]))

    def test_partition_rejects_duplicate_flipped_missing_foreign(self):
        for tris, owners in (([[0, 1, 2]]*2, [0, 0]),
                             ([[0, 2, 1], [0, 2, 3]], [0, 0]),
                             ([[0, 1, 2]], [0]),
                             ([[0, 1, 2], [0, 2, 4]], [0, 0]),
                             (self.triangles, [0, 1])):
            with self.subTest(tris=tris), self.assertRaises(ValueError):
                generalized_fixed_quad_triangles([[0, 1, 2, 3]], tris, owners)

    def test_no_categorical_type_coercion(self):
        rows = [[0, [1, 2]], [1, [1, 2]]]
        for values in ([1, '1'], [True, 1], [1., 1.]):
            with self.subTest(values=values), self.assertRaises(ValueError):
                generalized_source_values(rows, values, categorical=True)
        self.assertEqual(generalized_source_values(rows, ['1', '1'],
                                                  categorical=True), '1')

    def test_corner_id_types(self):
        for field in ('loop_vertices', 'loop_owners'):
            kwargs = dict(owner=4, layer='uv', loop_vertices=[0, 1, 2, 3],
                          loop_owners=[4]*4, loop_values=self.xyz[:, :2],
                          loop_ids=[0, 1, 2, 3])
            for invalid in (True, 0., '0'):
                bad = copy.deepcopy(kwargs)
                bad[field][0] = invalid
                with self.subTest(field=field, invalid=invalid), self.assertRaises(ValueError):
                    generalized_corner_values(self.rows, **bad)

    def test_entire_sparse_payload_finite(self):
        for operation in ('append', 'replace'):
            for value in (float('nan'), float('inf'), -float('inf')):
                record = copy.deepcopy(self.binding)
                record['arbitrary_nested_field'] = {'position': [value]}
                op = dict(operation=operation, identity='new' if operation == 'append'
                          else 'old', record=record)
                with self.subTest(operation=operation, value=value), self.assertRaises(ValueError):
                    generalized_sparse_readset({'old': self.binding},
                                               {'old': self.binding}, [op])

    def test_raw_mixed_types_not_coerced(self):
        rows = [[0, [1, 2]], [1, [1, 2]]]
        out = generalized_source_values(rows, [[1, 'x'], [1, 'x']], categorical=True)
        self.assertIs(type(out[0]), int)
        self.assertIs(type(out[1]), str)
        with self.assertRaises(ValueError):
            generalized_source_values(rows, [True, 0.])
        mixed = self.xyz.tolist()
        mixed[1][0] = True
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(mixed, [self.xyz], self.triangles)
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(self.xyz, [mixed], self.triangles)

    def test_source_transport_overflow_rejected(self):
        # Exercise the derived-result guard deterministically, not by assuming
        # that a particular BLAS reduction over finite donors must overflow.
        for nonfinite in (np.inf, -np.inf, np.nan):
            with self.subTest(nonfinite=nonfinite), patch(
                    'source_generalized_transport.np.tensordot',
                    return_value=np.array([0., nonfinite, 1.])) as arithmetic:
                with self.assertRaisesRegex(ValueError, '^Nonfinite derived source transport$'):
                    generalized_source_values(self.rows, self.xyz)
                arithmetic.assert_called_once()

    def test_source_transport_near_limit_is_portable(self):
        rows = [[i, [i + 1, 10]] for i in range(4)]
        values = np.full(4, np.finfo(float).max)
        # Exact rational weights give finite DBL_MAX. Rounded products/reduction
        # may instead overflow on some BLAS/CPUs. Observe this SAME arithmetic
        # call, rather than assuming either result or broad-catching any error.
        real_tensordot, derived = np.tensordot, []

        def capture(*args, **kwargs):
            result = real_tensordot(*args, **kwargs)
            derived.append(result)
            return result

        with patch('source_generalized_transport.np.tensordot', side_effect=capture):
            try:
                result = generalized_source_values(rows, values)
            except ValueError as error:
                self.assertEqual(str(error), 'Nonfinite derived source transport')
                self.assertEqual(len(derived), 1)
                self.assertFalse(np.isfinite(derived[0]).all())
            else:
                self.assertEqual(len(derived), 1)
                self.assertTrue(np.isfinite(derived[0]).all())
                np.testing.assert_array_equal(result, derived[0])

    def test_sparse_json_key_coercion_rejected(self):
        for key in (1, True, ''):
            with self.assertRaises(ValueError):
                generalized_sparse_readset({}, {key: self.binding}, [dict(
                    identity='1', operation='append', record=self.binding)])
        for key, string_key in ((1, '1'), (True, 'true')):
            with self.assertRaises(ValueError):
                generalized_sparse_readset({'v': {key: 'x'}},
                                           {'v': {string_key: 'x'}}, [])
            with self.assertRaises(ValueError):
                generalized_sparse_readset({'v': {string_key: 'x'}},
                                           {'v': {key: 'x'}}, [])
        with self.assertRaises(ValueError):
            generalized_sparse_readset({'v': (1, 2)}, {'v': [1, 2]}, [])

    def test_nonnumeric_geometry_rejected(self):
        for points in (self.xyz.astype(bool), self.xyz.astype(str)):
            with self.assertRaises(ValueError):
                generalized_fixed_triangle_poses(points, [points], self.triangles)
            with self.assertRaises(ValueError):
                generalized_fixed_triangle_poses(self.xyz, [points], self.triangles)

    def test_relative_cosine_underflow_rejected(self):
        neutral = [[0., 0., 0.], [0., 0., 1.], [0., -1e154, 0.]]
        pose = [[0., 0., 0.], [0., 0., 1.], [1e154, -1e-170, 0.]]
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(neutral, [pose], [[0, 1, 2]])

    def test_derived_overflow_fails_closed(self):
        planar = np.array([[0., 0., 0.], [1., 0., 0.],
                           [1., 1., 0.], [0., 1., 0.]])
        # Cross overflow, finite-cross/norm overflow, and subtraction overflow.
        for scale in (1e200, 1e100):
            huge = planar * scale
            with self.subTest(scale=scale), self.assertRaises(ValueError):
                generalized_fixed_triangle_poses(huge, [huge], self.triangles)
            with self.assertRaises(ValueError):
                generalized_fixed_triangle_poses(planar, [huge], self.triangles)
        extreme = planar.copy()
        extreme[0, 0] = -1e308
        extreme[1, 0] = 1e308
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(extreme, [extreme], self.triangles)

    def test_pose_scope_and_actual_reversal(self):
        result = generalized_fixed_triangle_poses(self.xyz, [self.xyz], self.triangles)
        self.assertEqual(result['states_checked'], 1)
        self.assertNotIn('729_pass', result)
        inverted = self.xyz.copy()
        inverted[:, 0] *= -1
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(self.xyz, [inverted], self.triangles)
        with self.assertRaises(ValueError):
            generalized_fixed_triangle_poses(self.xyz, [self.xyz * 0], self.triangles)


if __name__ == '__main__':
    unittest.main()
