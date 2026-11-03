"""Existing edge layer bindings, never BMesh allocation or coordinate matching."""
from types import SimpleNamespace
import unittest
from construction_markers import MARKERS
from construction_weld_edges import EDGE_MARKER, bind_edges, verify_edge_donors, attach_edge_mesh, clear_edge_mesh, unmarked_fields
from construction_markers import fields
from construction_topology import digest
from test_native_lifecycle import native


class Element:
    def __init__(self, marker, verts=()):
        self.marker, self.verts = marker, verts

    def __getitem__(self, layer):
        return self.marker


class Elements(list):
    def __init__(self, values, name):
        super().__init__(values)
        self.layers = SimpleNamespace(int={name: object()})


def fixture():
    points = [Element(1), Element(2), Element(3)]
    edges = [Element(1, points[:2]), Element(2, points[1:])]
    return SimpleNamespace(verts=Elements(points, MARKERS['POINT']), edges=Elements(edges, EDGE_MARKER))


class WeldEdgeBindingTests(unittest.TestCase):
    def test_mesh_marker_attach_cleanup_preserves_all_other_fields(self):
        obj = native()
        before = digest(fields(obj))
        attach_edge_mesh(obj)
        self.assertEqual(digest(unmarked_fields(obj)), before)
        self.assertEqual([v.value for v in obj.data.attributes[EDGE_MARKER].data], list(range(1, len(obj.data.edges) + 1)))
        clear_edge_mesh(obj)
        self.assertEqual(digest(fields(obj)), before)

    def test_mesh_marker_collision_is_preserved(self):
        obj = native()
        original = obj.data.attributes.new(EDGE_MARKER, 'INT', 'EDGE')
        with self.assertRaisesRegex(ValueError, 'collision'):
            attach_edge_mesh(obj)
        self.assertIs(obj.data.attributes.get(EDGE_MARKER), original)

    def test_bad_unrelated_fields_do_not_block_marker_cleanup(self):
        obj = native()
        attach_edge_mesh(obj)
        obj.data.attributes['NormalRegion'].data_type = 'UNKNOWN'
        with self.assertRaisesRegex(ValueError, 'Unsupported'):
            clear_edge_mesh(obj)
        self.assertIsNone(obj.data.attributes.get(EDGE_MARKER))

    def test_binds_existing_objects_and_full_ordinal_endpoints(self):
        bm = fixture()
        layer, original = bind_edges(bm, {'edges': [[0, 1], [1, 2]]})
        self.assertEqual(list(original), list(bm.edges))
        self.assertEqual(list(original.values()), [0, 1])
        receipt = verify_edge_donors(bm, layer, original, {'edges': [[0], [1]]})
        self.assertTrue(all(row['original_edge_object_survived'] for row in receipt['rows']))

    def test_missing_typed_storage_fails_closed(self):
        bm = fixture()
        bm.edges.layers.int = {}
        with self.assertRaisesRegex(ValueError, 'typed'):
            bind_edges(bm, {'edges': [[0, 1], [1, 2]]})

    def test_ordinal_endpoint_mismatch_fails_closed(self):
        bm = fixture()
        bm.edges[0].marker = 2
        with self.assertRaisesRegex(ValueError, 'identity/incidence'):
            bind_edges(bm, {'edges': [[0, 1], [1, 2]]})

    def test_original_edge_cannot_change_custom_data_donor(self):
        bm = fixture()
        layer, original = bind_edges(bm, {'edges': [[0, 1], [1, 2]]})
        bm.edges[0].marker = 2
        with self.assertRaisesRegex(ValueError, 'changed its custom-data donor'):
            verify_edge_donors(bm, layer, original, {'edges': [[0, 1], [1]]})


if __name__ == '__main__':
    unittest.main()
