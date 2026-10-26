"""Boundary donor ownership, ambiguity and transform regression checks."""

import copy
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import numpy as np
from ownership_test_fixtures import (
    KDTree, Matrix, Group, add_attribute, boundary_pair, mesh_object,
)
from source_weld_ownership import capture_oral_skin_donors, restore_oral_skin_donors


class WeldOwnershipTests(unittest.TestCase):
    def setUp(self):
        self.mock = patch.dict('sys.modules', {
            'mathutils.kdtree': SimpleNamespace(KDTree=KDTree),
        })
        self.mock.start()
        self.addCleanup(self.mock.stop)
        self.skin, self.oral = boundary_pair()

    def capture(self):
        return capture_oral_skin_donors([self.skin, self.oral], self.skin)

    def test_only_shared_boundary_fields_are_restored(self):
        snapshot = self.capture()
        self.assertEqual(len(snapshot['records']), 2)
        self.assertTrue(all(row['values']['BlinkSection'] == 0 for row in snapshot['records']))
        joined = mesh_object('Face', [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, -1, 0)],
                             [[0, 1, 2], [0, 1, 3]], [1, 10])
        joined.vertex_groups.append(Group('Continuous facial shell', 0, joined))
        add_attribute(joined, 'BlinkSection', 'INT', 'POINT', [9] * 4, 'value')
        uv = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] * 2
        add_attribute(joined, 'UVMap', 'FLOAT2', 'CORNER', uv, 'vector')
        positions = [vertex.co.copy() for vertex in joined.data.vertices]
        untouched = [joined.data.attributes['Color'].data[i].color for i in (2, 3)]
        result = restore_oral_skin_donors(joined, snapshot)
        self.assertEqual(result['boundary_vertices'], 2)
        self.assertEqual(result['group_memberships_changed'], 2)
        for vertex, before in zip(joined.data.vertices, positions):
            np.testing.assert_array_equal(vertex.co, before)
        self.assertEqual([joined.data.attributes['Color'].data[i].color for i in (2, 3)], untouched)
        self.assertEqual([item.value for item in joined.data.attributes['BlinkSection'].data], [0, 0, 9, 9])
        self.assertEqual([item.vector for item in joined.data.attributes['UVMap'].data], uv)
        self.assertTrue(all(joined.data.vertices[i].groups[0].weight == 1 for i in (0, 1)))

    def test_ambiguous_capture_is_rejected(self):
        self.oral.data.vertices[2].co = np.array([0, 0, 0])
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            self.capture()

    def test_ambiguous_restore_is_rejected(self):
        snapshot = self.capture()
        snapshot['records'][1]['position'] = copy.deepcopy(snapshot['records'][0]['position'])
        joined = mesh_object('Face', [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, -1, 0)],
                             [[0, 1, 2], [0, 1, 3]], [1, 10])
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            restore_oral_skin_donors(joined, snapshot)

    def test_different_axes_or_scale_are_rejected(self):
        scale = np.eye(4)
        scale[0, 0] = 1.1
        self.oral.matrix_world = Matrix(scale)
        with self.assertRaisesRegex(ValueError, 'identical object axes and scale'):
            self.capture()

    def test_translation_preserves_vector_fields(self):
        before = self.capture()
        target = mesh_object('target', [], [], [])
        transform = np.eye(4)
        transform[:3, 3] = [.2, -.3, .1]
        target.matrix_world = Matrix(transform)
        snapshot = capture_oral_skin_donors([self.skin, self.oral], target)
        self.assertEqual(len(snapshot['records']), 2)
        np.testing.assert_allclose(snapshot['records'][0]['position'], [-.2, .3, -.1])
        self.assertEqual(snapshot['records'][0]['values']['MouthOpenDelta'],
                         before['records'][0]['values']['MouthOpenDelta'])


if __name__ == '__main__':
    unittest.main()
