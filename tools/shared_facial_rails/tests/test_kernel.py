import copy
import math
import unittest
from .fixtures import grid, joint_grid, permute
from ..geometry import metrics, evaluate
from ..packet import Source, digest
from ..redistribute import initial, materialize, redistribute

class KernelTests(unittest.TestCase):

    def setUp(self):
        self.source = grid(4, 5)

    def test_two_genuinely_different_layouts(self):
        sizes = []
        for shape in ((4, 5), (5, 7)):
            source = grid(*shape)
            p = source.data
            sizes.append(len(p['faces']))
            r = redistribute(source, source.readset())
            for name, path in p['rails'].items():
                if all((v in p['protected'] for v in path)):
                    continue
                self.assertLess(r['after'][name]['ratio'], r['before'][name]['ratio'])
                self.assertLess(r['after'][name]['worst_relative'],
                    r['before'][name]['worst_relative'])
                self.assertLess(r['after'][name]['peak_turn'], r['before'][name]['peak_turn'])
            self.assertLessEqual(r['original_field_error'], 0.0005)
            self.assertLessEqual(r['max_step'], 0.003)
            self.assertFalse(r['native_ready'])
            self.assertFalse(r['full_model_success'])
            self.assertEqual(len(r['materialized']['keys']), 6)
            for owner, f in p['faces'].items():
                for corner in r['materialized']['corners'][owner]:
                    self.assertEqual(set(corner['uv']), set(p['uv_layers']))
            self.assertTrue(any((r['transverse_after'][n]['ratio'] > r['transverse_before'][n]['ratio'] for n in p['transverse'])))
        self.assertEqual(sizes, [12, 24])

    def test_identity_permutation_face_loop_and_owner_relabel(self):
        for shape in ((4, 5), (5, 7)):
            s = grid(*shape)
            other, vm, fm = permute(s)
            a = redistribute(s, s.readset())
            b = redistribute(other, other.readset())
            for uid in s.data['points']:
                self.assertEqual(a['materialized']['points'][uid],
                    b['materialized']['points'][vm[uid]])
                self.assertEqual(a['materialized']['fields'][uid],
                    b['materialized']['fields'][vm[uid]])
            for owner, f in s.data['faces'].items():
                otherface = other.data['faces'][fm[owner]]
                for j, uid in enumerate(f['vertices']):
                    k = otherface['vertices'].index(vm[uid])
                    self.assertEqual(a['materialized']['corners'][owner][j]['uv'],
                        b['materialized']['corners'][fm[owner]][k]['uv'])
            self.assertEqual(a['after'], b['after'])

    def test_protected_interface_and_endpoints_unchanged(self):
        s = self.source
        p = s.data
        r = redistribute(s, s.readset())
        before = materialize(s, initial(s))
        for uid in p['protected']:
            self.assertEqual(r['state']['rows'][uid], ((uid, 1.0),))
            self.assertEqual(r['materialized']['points'][uid], tuple(p['points'][uid]))
            self.assertEqual(r['materialized']['fields'][uid], before['fields'][uid])
        self.assertEqual(p, s.data)

    def test_independent_source_rows_all_keys(self):
        s = self.source
        p = s.data
        r = redistribute(s, s.readset())
        for uid, row in r['state']['rows'].items():
            for key in p['keys']:
                expected = evaluate({v: f[key] for v, f in p['frozen_fields'].items()}, row)
                self.assertEqual(r['materialized']['fields'][uid][key], expected)

    def test_corner_discontinuous_sides_retained(self):
        s = self.source
        r = redistribute(s, s.readset())
        uid = 'point-1-2'
        found = []
        for owner, f in s.data['faces'].items():
            for j, v in enumerate(f['vertices']):
                if v == uid:
                    found.append(r['materialized']['corners'][owner][j])
        self.assertEqual(len({x['side'] for x in found}), 2)
        self.assertEqual(len({tuple(x['uv']['base']) for x in found}), 2)

    def test_complete_incident_readset(self):
        s = self.source
        r = redistribute(s, s.readset())
        expected = {o for o,
            f in s.data['faces'].items() if set(f['vertices']) & set(r['changed_points'])}
        self.assertEqual(set(r['incident_faces']), expected)
        self.assertEqual(set(r['readset']['faces']), set(s.data['faces']))

    def test_stale_source_rejected(self):
        s = self.source
        p = s.data
        p['points']['point-1-1'][0] += 0.0001
        with self.assertRaisesRegex(ValueError, 'stale'):
            Source(p, s.sha256)

    def test_stale_readset_rejected(self):
        s = self.source
        b = s.readset()
        b['faces'].pop(next(iter(b['faces'])))
        with self.assertRaisesRegex(ValueError, 'stale'):
            redistribute(s, b)

    def test_rebased_fields_rejected(self):
        s = self.source
        r = redistribute(s, s.readset())
        state = copy.deepcopy(r['state'])
        state['baseline'] = digest(r['materialized']['fields'])
        with self.assertRaisesRegex(ValueError, 'rebased'):
            redistribute(s, s.readset(), state)

    def test_resume_keeps_original_baseline(self):
        s = self.source
        r = redistribute(s, s.readset())
        r2 = redistribute(s, s.readset(), r['state'])
        self.assertEqual(r2['state']['baseline'], initial(s)['baseline'])
        self.assertLessEqual(r2['original_field_error'], 0.0005)

    def test_ambiguous_rail_rejected(self):
        p = self.source.data
        p['rails']['alias'] = p['rails']['rail-1'][:]
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            Source(p, digest(p))

    def test_missing_side_rejected(self):
        p = self.source.data
        p['faces']['owner-1-1']['corners'][0]['side'] = ''
        with self.assertRaisesRegex(ValueError, 'side'):
            Source(p, digest(p))

    def test_missing_layer_rejected(self):
        p = self.source.data
        p['faces']['owner-1-1']['corners'][0]['uv'].pop('detail')
        with self.assertRaisesRegex(ValueError, 'layer'):
            Source(p, digest(p))

    def test_nonadjacent_donors_rejected(self):
        s = self.source
        state = initial(s)
        state['rows']['point-1-1'] = (('point-1-0', 0.5), ('point-1-3', 0.5))
        with self.assertRaisesRegex(ValueError, 'nonadjacent'):
            materialize(s, state)

    def test_protected_movement_rejected(self):
        s = self.source
        state = initial(s)
        state['rows']['point-0-1'] = (('point-0-0', 0.5), ('point-0-1', 0.5))
        with self.assertRaisesRegex(ValueError, 'protected'):
            materialize(s, state)

    def test_field_cap_cannot_be_relaxed(self):
        s = self.source
        for x in [0.00050001, float('inf'), float('nan'), 0]:
            with self.assertRaises(ValueError):
                redistribute(s, s.readset(), field_limit=x)

    def test_policy_bounds(self):
        s = self.source
        for x in [float('nan'), float('inf'), 0, -1]:
            with self.assertRaises(ValueError):
                redistribute(s, s.readset(), max_step=x)

    def test_actual_turn_orientation(self):
        p = {'a': (0, 0, 0), 'b': (1, 0, 0), 'c': (1, 1, 0)}
        self.assertEqual(metrics(p, ['a', 'b', 'c'])['peak_turn'], math.pi / 2)
        p['c'] = (0, 0, 0)
        self.assertEqual(metrics(p, ['a', 'b', 'c'])['peak_turn'], math.pi)

    def test_actual_field_drift_rejected(self):
        p = self.source.data
        for fields in p['frozen_fields'].values():
            for key, value in fields.items():
                fields[key] = [z * 1000 for z in value]
        s = Source(p, digest(p))
        with self.assertRaisesRegex(ValueError, 'no admitted'):
            redistribute(s, s.readset(), trials=1, max_step=0.01)

    def test_actual_local_motion_bound_rejected(self):
        s = self.source
        with self.assertRaisesRegex(ValueError, 'no admitted'):
            redistribute(s, s.readset(), trials=1, max_step=1e-06)

    def test_center_join_is_measured_when_in_full_path(self):
        p = {'left': (-1, 0.2, 0), 'center': (0, 0, 0), 'right': (1, 0.2, 0)}
        self.assertGreater(metrics(p, ['left', 'center', 'right'])['peak_turn'], 0.39)

    def test_wrong_corner_side_not_silently_nearest(self):
        from ..packet import corner_value
        with self.assertRaisesRegex(ValueError, 'missing corner side'):
            corner_value(self.source,
                (('point-1-1', 0.5), ('point-1-2', 0.5)),
                'not-a-side',
                'band-1')

    def test_wrong_category_not_interpolated(self):
        from ..packet import corner_value
        with self.assertRaisesRegex(ValueError, 'missing corner side'):
            corner_value(self.source,
                (('point-1-1', 0.5), ('point-1-2', 0.5)),
                'row-1:toward-1',
                'not-a-category')

    def test_degenerate_source_rail_rejected(self):
        p = self.source.data
        p['points']['point-1-2'] = p['points']['point-1-1'][:]
        with self.assertRaisesRegex(ValueError, 'degenerate'):
            Source(p, digest(p))

    def test_joint_policy_rejects_rail_only_support_regression(self):
        for shape in [(4, 5), (5, 7)]:
            source = grid(*shape)
            with self.assertRaisesRegex(ValueError, 'no admitted'):
                redistribute(source, source.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')

    def test_joint_policy_accepts_both_genuine_layouts(self):
        for shape in [(4, 5), (5, 7)]:
            source = joint_grid(*shape)
            p = source.data
            r = redistribute(source, source.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            self.assertTrue(r['synthetic_joint_admitted'])
            self.assertFalse(r['native_ready'])
            for n in p['rails']:
                self.assertLess(r['after'][n]['ratio'], r['before'][n]['ratio'])
                self.assertLess(r['after'][n]['peak_turn'], r['before'][n]['peak_turn'])
            for n in p['transverse']:
                self.assertLess(r['transverse_after'][n]['ratio'],
                    r['transverse_before'][n]['ratio'])
                self.assertLess(r['transverse_after'][n]['worst_relative'],
                    r['transverse_before'][n]['worst_relative'])
            self.assertLess(r['quad_after']['max_aspect'], r['quad_before']['max_aspect'])
            self.assertGreater(r['quad_after']['min_angle'], r['quad_before']['min_angle'])
            self.assertLess(r['quad_after']['max_angle'], r['quad_before']['max_angle'])
            for uid in p['protected']:
                self.assertEqual(tuple(p['points'][uid]), r['materialized']['points'][uid])

    def test_joint_policy_cannot_freeze_already_worsened_input(self):
        s = self.source
        r = redistribute(s, s.readset())
        with self.assertRaisesRegex(ValueError, 'frozen joint'):
            redistribute(s, s.readset(), r['state'], policy='SYNTHETIC_JOINT_NONREGRESSION_V1')

    def test_joint_result_permutation_invariance(self):
        for shape in [(4, 5), (5, 7)]:
            s = joint_grid(*shape)
            p, vm, fm = permute(s)
            a = redistribute(s, s.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            b = redistribute(p, p.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            for uid in s.data['points']:
                self.assertEqual(a['materialized']['points'][uid],
                    b['materialized']['points'][vm[uid]])
            self.assertEqual(a['after'], b['after'])
            self.assertEqual(a['transverse_after'], b['transverse_after'])

    def test_equal_content_raw_author_roles_are_valid(self):
        p = joint_grid(4, 5).data
        p['baseline_roles']['author']['content_sha256'] = p['baseline_roles']['raw']['content_sha256']
        p['baseline_roles']['author']['capture_id'] = p['baseline_roles']['raw']['capture_id']
        source = Source(p, digest(p))
        result = redistribute(source, source.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
        self.assertTrue(result['synthetic_joint_admitted'])
        self.assertEqual(result['state']['baseline_roles'], digest(p['baseline_roles']))

    def test_wrong_baseline_role_binding_rejected(self):
        p = self.source.data
        p['baseline_roles']['author']['role'] = 'raw'
        with self.assertRaisesRegex(ValueError, 'role binding'):
            Source(p, digest(p))

    def test_missing_baseline_role_rejected(self):
        p = self.source.data
        p['baseline_roles'].pop('raw')
        with self.assertRaisesRegex(ValueError, 'role records'):
            Source(p, digest(p))

    def test_malformed_baseline_content_hash_rejected(self):
        for value in ['abc', 'g' * 64, None, 1]:
            p = self.source.data
            p['baseline_roles']['raw']['content_sha256'] = value
            with self.assertRaisesRegex(ValueError, 'content hash'):
                Source(p, digest(p))

    def test_stale_baseline_role_readset_rejected(self):
        source = self.source
        binding = source.readset()
        binding['baseline_roles'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'stale'):
            redistribute(source, binding)

    def test_resumed_baseline_role_rebasing_rejected(self):
        source = self.source
        state = initial(source)
        state['baseline_roles'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'rebased'):
            redistribute(source, source.readset(), state)

    def test_along_relative_regression_rejected_despite_better_ratio(self):
        from ..redistribute import rail_nonworse
        a = metrics({str(j): (x, 0, 0) for j, x in enumerate([0, 1, 3, 5])}, ['0', '1', '2', '3'])
        b = metrics({str(j): (x, 0, 0) for j, x in enumerate([0, 1.2, 2.4, 4.7])},
            ['0', '1', '2', '3'])
        self.assertLess(b['ratio'], a['ratio'])
        self.assertGreater(b['worst_relative'], a['worst_relative'])
        self.assertFalse(rail_nonworse(b, a))

    def test_empty_source_domains_rejected(self):
        for key in ['points', 'faces', 'rails', 'transverse']:
            p = self.source.data
            p[key] = {}
            with self.assertRaisesRegex(ValueError, 'empty'):
                Source(p, digest(p))

    def test_empty_transverse_group_rejected(self):
        p = self.source.data
        p['transverse']['gap-0'] = []
        with self.assertRaisesRegex(ValueError, 'transverse'):
            Source(p, digest(p))

    def test_duplicate_transverse_edge_rejected(self):
        p = self.source.data
        p['transverse']['gap-0'].append(p['transverse']['gap-0'][0])
        with self.assertRaisesRegex(ValueError, 'transverse'):
            Source(p, digest(p))

    def test_nonfinite_source_packet_rejected(self):
        for value in [float('nan'), float('inf'), float('-inf')]:
            p = self.source.data
            p['points']['point-1-1'][0] = value
            with self.assertRaises(ValueError):
                Source(p, self.source.sha256)

    def test_finite_but_overflowing_derived_geometry_rejected(self):
        p = self.source.data
        p['points']['point-1-1'][0] = 1e+308
        with self.assertRaisesRegex(ValueError, 'nonfinite derived'):
            Source(p, digest(p))

    def test_collapsed_fixed_triangle_rejected(self):
        p = self.source.data
        p['points']['point-1-1'] = p['points']['point-0-0'][:]
        with self.assertRaisesRegex(ValueError, 'degenerate fixed triangle'):
            Source(p, digest(p))

    def test_saved_keys_overflow_independent_reproduction(self):
        for sign in (1, -1):
            p = joint_grid(4, 5).data
            for uid in p['points']:
                p['points'][uid][2] = sign * 1e+308
                for value in p['frozen_fields'][uid].values():
                    value[2] = sign * 1e+308
            with self.assertRaisesRegex(ValueError, 'nonfinite derived'):
                Source(p, digest(p))

    def test_large_finite_saved_keys_not_arbitrarily_clipped(self):
        p = joint_grid(4, 5).data
        for uid in p['points']:
            p['points'][uid][2] = 8e+307
            for value in p['frozen_fields'][uid].values():
                value[2] = 8e+307
        source = Source(p, digest(p))
        out = materialize(source, initial(source))
        for values in out['keys'].values():
            for point in values.values():
                self.assertEqual(point[2], 1.6e+308)

    def test_public_corner_helper_rejects_invalid_weights(self):
        from ..packet import corner_value
        a, b = ('point-1-1', 'point-1-2')
        for row in [((a, 2), (b, -1)),
            ((a, 0.4), (b, 0.4)),
            ((a, 0.5), (b, 0.5000000000000002)),
            ((a, float('nan')), (b, 0.5)),
            ((a, float('inf')), (b, 0.5)),
            ((a, True),),
            ((a, 0), (b, 1)),
            ((a, 0.5), (a, 0.5)),
            (('unknown', 1),),
            (),
            ((a, '1'),)]:
            with self.assertRaises(ValueError):
                corner_value(self.source, row, 'row-1:toward-1', 'band-1')

    def test_public_corner_helper_rejects_nonadjacent_donors(self):
        from ..packet import corner_value
        with self.assertRaisesRegex(ValueError, 'nonadjacent'):
            corner_value(self.source,
                (('point-1-0', 0.5), ('point-1-3', 0.5)),
                'row-1:toward-1',
                'band-1')

    def test_public_interpolation_rejects_nonfinite_values(self):
        for value in [float('nan'), float('inf'), float('-inf')]:
            with self.assertRaises(ValueError):
                evaluate({'a': (value, 0, 0)}, (('a', 1),))

    def test_public_interpolation_rejects_extrapolation(self):
        with self.assertRaisesRegex(ValueError, 'nonconvex'):
            evaluate({'a': (1e+308, 0, 0), 'b': (-1e+308, 0, 0)}, (('a', 2), ('b', -1)))

    def test_all_materialized_channels_finite(self):
        for shape in [(4, 5), (5, 7)]:
            source = joint_grid(*shape)
            r = redistribute(source, source.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            m = r['materialized']
            arrays = list(m['points'].values())
            arrays.extend((value for fields in m['fields'].values() for value in fields.values()))
            arrays.extend((value for key in m['keys'].values() for value in key.values()))
            arrays.extend((value for corners in m['corners'].values() for corner in corners for value in corner['uv'].values()))
            self.assertTrue(all((math.isfinite(x) for value in arrays for x in value)))

    def test_along_edge_cannot_masquerade_as_transverse(self):
        p = self.source.data
        p['transverse']['gap-0'][0] = ['point-0-0', 'point-0-1']
        with self.assertRaisesRegex(ValueError, 'transverse role'):
            Source(p, digest(p))

    def test_transverse_group_cannot_be_cropped(self):
        p = self.source.data
        p['transverse']['gap-0'].pop()
        with self.assertRaisesRegex(ValueError, 'incomplete transverse'):
            Source(p, digest(p))

    def test_distinct_rail_pairs_cannot_be_mixed(self):
        p = self.source.data
        p['transverse']['gap-0'] += p['transverse'].pop('gap-1')
        with self.assertRaisesRegex(ValueError, 'mixed transverse'):
            Source(p, digest(p))

    def test_duplicate_cross_group_edges_rejected(self):
        p = self.source.data
        p['transverse']['alias'] = p['transverse']['gap-0'][:]
        with self.assertRaisesRegex(ValueError, 'duplicate transverse'):
            Source(p, digest(p))

    def test_immutable_packet_copy(self):
        p = self.source.data
        p['points'].clear()
        self.assertTrue(self.source.data['points'])
if __name__ == '__main__':
    unittest.main()
