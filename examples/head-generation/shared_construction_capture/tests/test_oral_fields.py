"""Exact permitted oral field effects; no native geometry/identity proof claimed."""
from copy import deepcopy
import unittest
from unittest.mock import patch
from construction_topology import digest
from construction_oral_fields import donor_schemas, expected_transfer, verify_oral_transfer


def mesh(count, faces, regions, skin=False):
    edges = sorted({tuple(sorted((a, b))) for face in faces for a, b in zip(face, face[1:] + face[:1])})
    def attr(domain, kind, values):
        return {'domain': domain, 'type': kind, 'values': values}
    return {'vertices': [[[i * .01, 0.0, 0.0], False, False, [[0, 1.0]] if skin else []] for i in range(count)],
            'faces': [[face] for face in faces], 'edges': [[list(edge)] for edge in edges],
            'groups': [[0, 'Continuous facial shell']], 'editor': {'selected': False},
            'attributes': {'Color': attr('POINT', 'FLOAT_COLOR', [[.9 if skin else .2, .5, .5, 1.0] for _ in range(count)]),
                           'NormalRegion': attr('FACE', 'INT', regions),
                           'crease_edge': attr('EDGE', 'FLOAT', [1.0 if skin else 0.0] * len(edges)),
                           'FacialCreaseGroup': attr('EDGE', 'INT', [3 if skin else 0] * len(edges)),
                           'UVMap': attr('CORNER', 'FLOAT2', [[.1, .2] for _ in range(sum(map(len, faces)))])}}


def fixture():
    faces = [[i, 4 + i, 4 + (i + 1) % 4, (i + 1) % 4] for i in range(4)] + [[4, 5, 6, 7]]
    skin, oral = mesh(4, [[0, 1, 2, 3]], [1], True), mesh(8, faces, [10] * 5)
    oral['attributes']['AbsentSkin'] = {'domain': 'POINT', 'type': 'FLOAT_VECTOR', 'values': [[1.0] * 3 for _ in range(8)]}
    before = mesh(8, [[0, 1, 2, 3]] + faces, [1] + [10] * 5)
    before['attributes']['AbsentSkin'] = deepcopy(oral['attributes']['AbsentSkin'])
    captured = {'skin_fields': skin, 'oral_fields': oral, 'skin_point_ids': ['s' + str(i) for i in range(4)],
                'point_schemas': donor_schemas([skin, oral])}
    targets = {'rows': [{'skin_point_id': 's' + str(i), 'oral_index': i, 'target_index': i} for i in range(4)]}
    return before, captured, targets


class OralFieldTests(unittest.TestCase):
    def test_only_declared_donor_fields_change(self):
        before, captured, targets = fixture()
        original = deepcopy((before, captured))
        expected, edges = expected_transfer(before, captured, targets)
        self.assertEqual((before, captured), original)
        self.assertEqual(len(edges), 4)
        for i in range(4):
            self.assertEqual(expected['attributes']['Color']['values'][i], [.9, .5, .5, 1.0])
            self.assertEqual(expected['attributes']['AbsentSkin']['values'][i], [0.0] * 3)
            self.assertEqual(expected['vertices'][i][3], [[0, 1.0]])
        self.assertEqual(expected['vertices'][4:], before['vertices'][4:])
        self.assertEqual(expected['attributes']['UVMap'], before['attributes']['UVMap'])
        self.assertEqual(expected['faces'], before['faces'])

    def test_rejects_wrong_schema_missing_group_or_boundary(self):
        for mutation in (lambda c: c['skin_fields']['attributes']['crease_edge'].__setitem__('type', 'INT'),
                         lambda c: c['skin_fields']['vertices'][0].__setitem__(3, []),
                         lambda c: c['oral_fields']['faces'].pop(0)):
            before, captured, targets = fixture()
            mutation(captured)
            with self.assertRaises(ValueError):
                expected_transfer(before, captured, targets)

    def test_restoration_requires_exact_full_expected_state(self):
        before, captured, targets = fixture()
        expected, _ = expected_transfer(before, captured, targets)
        captured['boundary'] = {}
        captured['join_source_sha256'] = 'join'
        captured['sha256'] = digest(captured)
        weld = {'after_native_fields': before}
        weld['sha256'] = digest(weld)
        packet = {'oral_join': captured, 'native_join': {'source_sha256': 'join'}, 'native_weld': weld}
        with patch('construction_oral_fields.compose_donor_targets', return_value=targets), patch('construction_oral_fields.fields', side_effect=[before, expected]):
            self.assertEqual(verify_oral_transfer(object(), [packet], {}, lambda *_: {'original': True}), {'original': True})
        self.assertFalse(packet['oral_transfer']['current_role_admission'])
        for mutate in (lambda s: s['vertices'][0][0].__setitem__(0, 99.0),
                       lambda s: s['attributes']['UVMap']['values'][0].__setitem__(0, .9),
                       lambda s: s['attributes']['Color']['values'][7].__setitem__(0, .9),
                       lambda s: s['editor'].__setitem__('selected', True)):
            actual = deepcopy(expected)
            mutate(actual)
            with patch('construction_oral_fields.compose_donor_targets', return_value=targets), patch('construction_oral_fields.fields', side_effect=[before, actual]):
                with self.assertRaisesRegex(ValueError, 'identity-derived donor policy'):
                    verify_oral_transfer(object(), [packet], {}, lambda *_: {})


if __name__ == '__main__':
    unittest.main()
