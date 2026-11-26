"""Private pure topology tests; never execute Blender or infer positional IDs."""
import unittest
from copy import deepcopy
from ear_contract import bind_ear_attachment, compose_attachment_targets
from construction_oral_contract import bind_oral_ring
from construction_topology import digest
from construction_weld_contract import terminal_map, projected_face, validate_weld


def ear_fixture(side):
    skin = {'point_ids': [f'skin{side}:{i}' for i in range(14)],
            'faces': [[i, i + 1, i + 8, i + 7] for i in range(6)]}
    ear = {'point_ids': [f'ear{side}:{i}' for i in range(14)],
           'faces': [[i + 1, i, i + 7, i + 8] for i in range(6)]}
    kwargs = {'side': side, 'root_source_indices': [1, 2, 3, 4, 5],
              'ear_rows': [[1], [2, 9], [3, 10], [4, 11], [5]],
              'lower_bridge': {'source_index': 0, 'ear_index': 0},
              'upper_bridge': {'source_index': 6, 'ear_index': 6}}
    return skin, ear, kwargs


def weld(before, pairs):
    targets = terminal_map(len(before['point_ids']), pairs)
    origins = sorted(set(targets.values()))
    def after(order):
        indices = {v: i for i, v in enumerate(order)}
        faces = []
        for i, face in enumerate(before['faces']):
            projected = projected_face(face['points'], face['corners'], targets)
            if len(projected) >= 3:
                faces.append({'face_origin': i, 'points': [indices[v] for _, v in projected],
                              'corner_origins': [c for c, _ in projected]})
        edges = sorted({tuple(sorted((indices[targets[a]], indices[targets[b]])))
                        for a, b in before['edges'] if targets[a] != targets[b]})
        return {'point_origins': order, 'faces': faces, 'edges': edges}
    native, post = after(origins), after(list(reversed(origins)))
    receipt = {'before': before, 'native_target_pairs': pairs, 'after_weld': native, 'after_mesh': post,
               'quotient': validate_weld(before, pairs, native), 'mesh_quotient': validate_weld(before, pairs, post)}
    receipt['sha256'] = digest(receipt)
    return receipt


def combined_fixture():
    ears, meshes = [], []
    for side in (-1, 1):
        skin, ear, kwargs = ear_fixture(side)
        ears.append(bind_ear_attachment(skin, ear, **kwargs))
        meshes.extend((skin, ear))
    oral = bind_oral_ring(['a', 'b', 'c'], ['a', 'c'], 'oral-shell',
                         ['oral:' + str(i) for i in range(8)], [0, 1, 2, 3])
    mouth_skin = {'point_ids': [r['skin_point_id'] for r in oral['pairs']], 'faces': [[0, 1, 2, 3]]}
    mouth = {'point_ids': oral['oral_point_ids'],
             'faces': [[(i + 1) % 4, i, i + 4, (i + 1) % 4 + 4] for i in range(4)] + [[7, 6, 5, 4]]}
    meshes.extend((mouth_skin, mouth))
    source = {'points': [p for mesh in meshes for p in mesh['point_ids']],
              'faces': [{'point_ids': [mesh['point_ids'][i] for i in f]} for mesh in meshes for f in mesh['faces']]}
    permutation = list(reversed(range(1, len(source['points']) + 1)))
    join = {'source': source, 'source_sha256': digest(source), 'identity_maps': {'POINT': permutation}}
    ids = [source['points'][i - 1] for i in permutation]
    lookup = {p: i for i, p in enumerate(ids)}
    faces, offset = [], 0
    for face in source['faces']:
        points = [lookup[p] for p in face['point_ids']]
        faces.append({'points': points, 'corners': list(range(offset, offset + len(points)))})
        offset += len(points)
    edges = sorted({tuple(sorted((a, b))) for f in faces for a, b in zip(f['points'], f['points'][1:] + f['points'][:1])})
    before = {'point_ids': ids, 'faces': faces, 'edges': edges, 'join_source_sha256': join['source_sha256']}
    pairs = [[lookup[r['skin_point_id']], lookup[r['oral_point_id']]] for r in oral['pairs']]
    pairs.extend([lookup[r['skin_point_id']], lookup[r['ear_point_id']]] for ear in ears for r in ear['pairs'])
    return oral, ears, join, weld(before, pairs)


class EarContractTests(unittest.TestCase):
    def test_roots_and_both_bridges_explicit(self):
        skin, ear, kwargs = ear_fixture(-1)
        result = bind_ear_attachment(skin, ear, **kwargs)
        self.assertEqual([r['evaluated_source_index'] for r in result['pairs']], list(range(7)))
        self.assertEqual([r['allocation_kind'] for r in result['pairs']], ['lower_bridge'] + ['root_row'] * 5 + ['upper_bridge'])
        self.assertFalse(result['current_role_admission'])

    def test_bad_rows_side_or_bridge_fail(self):
        for mutation in ('side', 'roots', 'rows', 'lower', 'upper'):
            skin, ear, kwargs = ear_fixture(1)
            if mutation == 'side': kwargs['side'] = True
            if mutation == 'roots': kwargs['root_source_indices'][2] = 1
            if mutation == 'rows': kwargs['ear_rows'][2][0] = 1
            if mutation == 'lower': kwargs['lower_bridge']['ear_index'] = 1
            if mutation == 'upper': kwargs['upper_bridge']['source_index'] = 0
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                bind_ear_attachment(skin, ear, **kwargs)

    def test_chain_order_orientation_and_incidence_fail(self):
        for mutation in ('order', 'orientation', 'duplicate', 'missing', 'chord'):
            skin, ear, kwargs = ear_fixture(1)
            if mutation == 'order': kwargs['root_source_indices'][1:3] = [3, 2]
            if mutation == 'orientation': ear['faces'] = [list(reversed(f)) for f in ear['faces']]
            if mutation == 'duplicate': skin['faces'].append(skin['faces'][0])
            if mutation == 'missing': ear['faces'].pop(0)
            if mutation == 'chord': skin['faces'].append([0, 3, 13])
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                bind_ear_attachment(skin, ear, **kwargs)

    def test_combined_permuted_native_and_mesh_storage(self):
        inputs = combined_fixture()
        original = deepcopy(inputs)
        result = compose_attachment_targets(*inputs)
        self.assertEqual(inputs, original)
        self.assertEqual(len(result['rows']), 18)
        self.assertEqual(len({r['target_index'] for r in result['rows']}), 18)
        self.assertEqual({r['component'] for r in result['rows']}, {'oral', 'ear:-1', 'ear:1'})
        for field in ('solver_ready', 'field_transfer_admission', 'current_role_admission'):
            self.assertFalse(result[field])

    def test_missing_extra_and_unrelated_merges_fail(self):
        for mutation in ('missing', 'extra', 'unrelated'):
            oral, ears, join, native = combined_fixture()
            pairs = deepcopy(native['native_target_pairs'])
            ids = native['before']['point_ids']
            if mutation == 'missing': pairs.pop()
            if mutation == 'extra': pairs.append([ids.index('ear-1:13'), pairs[0][1]])
            if mutation == 'unrelated': pairs.append([ids.index('skin-1:13'), ids.index('skin1:13')])
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                compose_attachment_targets(oral, ears, join, weld(native['before'], pairs))

    def test_bilateral_receipt_or_identity_alias_fails(self):
        oral, ears, join, native = combined_fixture()
        with self.assertRaises(ValueError):
            compose_attachment_targets(oral, [ears[0], ears[0]], join, native)
        ears[0]['pairs'][0]['skin_point_id'] = ears[1]['pairs'][0]['skin_point_id']
        with self.assertRaises(ValueError):
            compose_attachment_targets(oral, ears, join, native)

    def test_join_weld_or_receipt_tampering_fails(self):
        for mutation in ('point_map', 'face_map', 'seal', 'post'):
            oral, ears, join, native = combined_fixture()
            if mutation == 'point_map': join['identity_maps']['POINT'][0] = True
            if mutation == 'face_map': join['source']['faces'][0]['point_ids'].reverse();join['source_sha256'] = digest(join['source'])
            if mutation == 'seal': native['sha256'] = 'forged'
            if mutation == 'post': native['after_mesh']['point_origins'].reverse()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                compose_attachment_targets(oral, ears, join, native)

    def test_resealed_declared_face_must_exist_in_actual_join(self):
        oral, ears, join, native = combined_fixture()
        skin, ear, kwargs = ear_fixture(-1)
        # An unchanged boundary does not authorize an invented interior face.
        skin['faces'].append([7, 9, 13])
        ears[0] = bind_ear_attachment(skin, ear, **kwargs)
        with self.assertRaisesRegex(ValueError, 'face incidence absent'):
            compose_attachment_targets(oral, ears, join, native)

    def test_native_survivor_can_be_skin_or_component(self):
        oral, ears, join, native = combined_fixture()
        reversed_pairs = [pair[::-1] for pair in native['native_target_pairs']]
        result = compose_attachment_targets(oral, ears, join, weld(native['before'], reversed_pairs))
        self.assertEqual(len(result['rows']), 18)


if __name__ == '__main__':
    unittest.main()
