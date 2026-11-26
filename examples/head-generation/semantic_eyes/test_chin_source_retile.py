"""Semantic capture and frozen field ownership regressions, without Blender."""
import unittest
from types import SimpleNamespace
import numpy as np
from chin_source_roles import capture_chin_source_roles, bind_chin_source_roles
from chin_source_fields import chin_interpolate_point_fields
from chin_source_template import chin_retile_recipe
from chin_source_contract import chin_graph, chin_opposite
from chin_source_shape import (chin_refine_selected_export, chin_shape_exact_distance,
                               chin_shape_tessellate)


def source_fixture():
    recipe = chin_retile_recipe()
    required = {name for face in recipe['old_faces'] for name in face}
    names = sorted(required | {'chin_row_3_station_12'})
    roles = {name: index for index, name in enumerate(names)}
    vertices = np.array([[.001 * (i + 1), -.1, 1.5 + .0001 * i] for i in range(len(names))])
    faces = [[roles[name] for name in face] for face in recipe['old_faces']]
    rows = [[roles[f'chin_row_{r}_station_{k}'] for k in range(count)]
            for r, count in enumerate((5, 7, 9, 13))]
    units = [[roles[f'chin_transition_{n}_{part}'] for part in ('left', 'middle', 'right')]
             for n in range(4)]
    outer = {'lower_transition': {'vertex_ids':
             [roles[f'outer_lower_transition_{part}'] for part in ('left', 'middle', 'right')]}}
    ring3 = [roles[f'mouth_ring_3_station_{k}'] for k in range(3)]
    mouth = [{'rings': [[], [], [], ring3, []]}]
    return vertices, faces, rows, units, outer, mouth, recipe


class ChinSemanticCaptureTests(unittest.TestCase):
    def test_capture_uses_only_current_face_union(self):
        args = source_fixture()
        result = capture_chin_source_roles(*args)
        self.assertEqual(len(result['positions']), 51)
        self.assertNotIn('chin_row_3_station_12', result['positions'])
        self.assertEqual(result['face_count'], 37)

    def test_capture_rejects_reversed_source_face(self):
        args = list(source_fixture())
        args[1][0] = args[1][0][::-1]
        with self.assertRaisesRegex(ValueError, 'oriented source disk'):
            capture_chin_source_roles(*args)

    def test_capture_rejects_transition_alias(self):
        args = list(source_fixture())
        args[3][1] = args[3][0]
        with self.assertRaisesRegex(ValueError, 'alias'):
            capture_chin_source_roles(*args)

    def test_bind_both_halves_after_float32_assembly(self):
        args = source_fixture()
        authored = capture_chin_source_roles(*args)
        v = args[0].astype(np.float32)
        v[:, 2] = np.float32(v[:, 2].astype(float) - .021)
        mirror = v.copy()
        mirror[:, 0] *= -1
        n = len(v)
        vertices = np.vstack((v, mirror)).astype(float)
        faces = args[1] + [[i + n for i in reversed(face)] for face in args[1]]
        result = bind_chin_source_roles(vertices, faces, [1] * 74, authored, args[-1], -.021)
        self.assertEqual([len(row['roles']) for row in result], [51, 51])
        self.assertEqual([len(row['face_ids']) for row in result], [37, 37])
        # Array positions are not part of the semantic contract.
        permutation = np.random.default_rng(17).permutation(len(vertices))
        inverse = np.argsort(permutation)
        permuted_faces = [[int(inverse[i]) for i in face] for face in faces]
        remapped = bind_chin_source_roles(vertices[permutation], permuted_faces,
                                          [1] * 74, authored, args[-1], -.021)
        for old, new in zip(result, remapped):
            for name, index in old['roles'].items():
                self.assertEqual(new['roles'][name], inverse[index])
        vertices[1] = vertices[0]
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            bind_chin_source_roles(vertices, faces, [1] * 74, authored, args[-1], -.021)


class ChinFrozenFieldTests(unittest.TestCase):
    def test_join_flag_is_structural_not_a_sampled_average(self):
        layer = SimpleNamespace(name='SmoothChinJoin')
        fields = [('int', layer, [0, 0, 1, 0])]
        result = chin_interpolate_point_fields(0, {1: .5, 2: .5}, fields)
        self.assertEqual(result[0][1], 0)
        with self.assertRaisesRegex(ValueError, 'chin-join member'):
            chin_interpolate_point_fields(2, {1: .5, 2: .5}, fields)

    def test_other_discrete_field_conflict_still_rejected(self):
        layer = SimpleNamespace(name='BlinkSection')
        with self.assertRaisesRegex(ValueError, 'discrete'):
            chin_interpolate_point_fields(0, {1: .5, 2: .5}, [('int', layer, [0, 0, 1])])

    def test_motion_uses_frozen_values_without_mutation(self):
        layer = SimpleNamespace(name='MouthOpenDelta')
        values = [(0., 0., 0.), (1., 2., 3.), (3., 4., 5.)]
        result = chin_interpolate_point_fields(0, {1: .25, 2: .75},
                                              [('float_vector', layer, values)])
        self.assertEqual(result[0][1], (2.5, 3.5, 4.5))
        self.assertEqual(values[0], (0., 0., 0.))

    def test_bad_donor_weights_fail_closed(self):
        for weights in ({}, {0: -.1, 1: 1.1}, {0: .4}, {0: float('nan')}):
            with self.assertRaisesRegex(ValueError, 'weights'):
                chin_interpolate_point_fields(0, weights, [])

    def test_opposite_edge_is_not_a_chart_row_guess(self):
        faces = [(0, 1, 4, 3), (1, 2, 5, 4), (3, 4, 7, 6), (4, 5, 8, 7)]
        adjacency, incident, _ = chin_graph(faces)
        self.assertEqual(chin_opposite(faces, adjacency, incident, 1, 4), 7)
        self.assertEqual(chin_opposite(faces, adjacency, incident, 3, 4), 5)
        self.assertIsNone(chin_opposite(faces, adjacency, incident, 0, 1))


class ChinShapeGateTests(unittest.TestCase):
    def test_passing_dense_proposal_is_identity_noop(self):
        proposal = {'new_vertices': [[1., 2., 3.]], 'lineage': {}}
        audit = {'pass': True, 'witnesses': [], 'cases': [{'max_mm': .93}]}
        context = SimpleNamespace(n=32, initial=proposal['new_vertices'], lineage={},
                                  audit=lambda points, lineage: audit)
        result, report = chin_refine_selected_export(context, proposal)
        self.assertIs(result, proposal)
        self.assertFalse(report['changed'])
        self.assertEqual(report['verification_sampling'], 32)

    def test_exact_triangle_distance_keeps_both_directions_meaningful(self):
        vertices = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        queries = np.array([[.25, .25, 1.], [0., 0., 0.], [-1., 0., 0.]])
        distances = chin_shape_exact_distance(queries, vertices, [[0, 1, 2]])
        np.testing.assert_allclose(distances, [1., 0., 1.], atol=1e-12)

    def test_surface_samples_follow_bilinear_not_chosen_diagonal(self):
        vertices = np.array([[0., 0., 0.], [1., 0., 0.], [1., 1., 1.], [0., 1., 0.]])
        points, triangles = chin_shape_tessellate(vertices, [[0, 1, 2, 3]], 2)
        np.testing.assert_array_equal(points[4], [.5, .5, .25])
        self.assertEqual(triangles.shape, (8, 3))


if __name__ == '__main__':
    unittest.main()
