"""Independent regression for complete source-derived rail-pair measurements."""
import copy, json, random, unittest
from pathlib import Path
from .fixtures import joint_grid, grid, permute
from ..packet import Source, digest
from ..redistribute import redistribute
from ..geometry import edge_metrics

class CanonicalPairTests(unittest.TestCase):

    def source(self, p):
        return Source(p, digest(p))

    def split(self, p):
        p['transverse'] = {f'{n}-{i}': [e] for n,
            ee in p['transverse'].items() for i,
            e in enumerate(ee)}
        return p

    def test_original_false_admission_fixture_rejected(self):
        p = json.loads((Path(__file__).parent / 'split-false-admission-payload.json').read_text())
        with self.assertRaisesRegex(ValueError, 'split transverse'):
            self.source(p)

    def test_singleton_partition_rejected_on_both_layouts(self):
        for shape in [(4, 5), (5, 7)]:
            with self.assertRaisesRegex(ValueError, 'split transverse'):
                self.source(self.split(joint_grid(*shape).data))

    def test_two_nontrivial_subgroups_rejected(self):
        p = joint_grid(5, 7).data
        ee = p['transverse'].pop('gap-0')
        p['transverse']['a'] = ee[:3]
        p['transverse']['b'] = ee[3:]
        with self.assertRaisesRegex(ValueError, 'split transverse'):
            self.source(p)

    def test_rename_and_reverse_group_order_preserves_metrics(self):
        for shape in [(4, 5), (5, 7)]:
            s = joint_grid(*shape)
            p = s.data
            names = {n: 'display-' + str(i) for i, n in enumerate(p['transverse'])}
            p['transverse'] = {names[n]: [e[::-1] for e in ee[::-1]] for n,
                ee in reversed(list(p['transverse'].items()))}
            t = self.source(p)
            a = redistribute(s, s.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            b = redistribute(t, t.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
            self.assertEqual(a['state']['rows'], b['state']['rows'])
            for n in names:
                for phase in ['before', 'after']:
                    self.assertEqual(a['transverse_' + phase][n],
                        b['transverse_' + phase][names[n]])

    def test_complete_domains_ignore_caller_edge_order(self):
        s = joint_grid(5, 7)
        p = s.data
        rng = random.Random(59)
        for ee in p['transverse'].values():
            rng.shuffle(ee)
        t = self.source(p)
        self.assertEqual(s.transverse_domains(), t.transverse_domains())

    def test_each_derived_domain_equals_actual_source_edges(self):
        s = joint_grid(5, 7)
        p = s.data
        roles = {uid: n for n, path in p['rails'].items() for uid in path}
        actual = set()
        for f in p['faces'].values():
            vv = f['vertices']
            actual.update((frozenset((a, b)) for a,
                b in zip(vv, vv[1:] + vv[:1]) if roles[a] != roles[b]))
        domains = s.transverse_domains()
        self.assertEqual(actual, {frozenset(e) for ee in domains.values() for e in ee})
        self.assertEqual(sum(map(len, domains.values())), len(actual))

    def test_duplicate_complete_pair_rejected(self):
        p = joint_grid(4, 5).data
        p['transverse']['alias'] = copy.deepcopy(p['transverse']['gap-0'])
        with self.assertRaisesRegex(ValueError, 'duplicate transverse'):
            self.source(p)

    def test_missing_complete_pair_rejected(self):
        p = joint_grid(4, 5).data
        p['transverse'].pop('gap-1')
        with self.assertRaisesRegex(ValueError, 'incomplete transverse'):
            self.source(p)

    def test_missing_station_rejected(self):
        p = joint_grid(4, 5).data
        p['transverse']['gap-1'].pop(2)
        with self.assertRaisesRegex(ValueError, 'incomplete transverse'):
            self.source(p)

    def test_disconnected_rail_jump_rejected(self):
        p = joint_grid(4, 5).data
        p['rails']['rail-1'].pop(2)
        with self.assertRaisesRegex(ValueError, 'rail not actual edges'):
            self.source(p)

    def test_identity_permutation_preserves_derived_metrics(self):
        s = joint_grid(5, 7)
        t, vm, fm = permute(s)
        a = redistribute(s, s.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
        b = redistribute(t, t.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
        self.assertEqual(a['transverse_before'], b['transverse_before'])
        self.assertEqual(a['transverse_after'], b['transverse_after'])

    def test_saved_key_overflow_remains_rejected(self):
        p = joint_grid(4, 5).data
        for v in p['points'].values():
            v[2] = 1e+308
        for fields in p['frozen_fields'].values():
            for v in fields.values():
                v[2] = 1e+308
        with self.assertRaises(ValueError):
            self.source(p)

    def test_original_regression_fixture_with_canonical_groups_not_admitted(self):
        p = json.loads((Path(__file__).parent / 'split-false-admission-payload.json').read_text())
        canonical = {}
        for name, edges in p['transverse'].items():
            canonical.setdefault(name.rsplit('-', 1)[0], []).extend(edges)
        p['transverse'] = canonical
        s = self.source(p)
        with self.assertRaisesRegex(ValueError, 'no admitted'):
            redistribute(s, s.readset(), policy='SYNTHETIC_JOINT_NONREGRESSION_V1')
if __name__ == '__main__':
    unittest.main()
