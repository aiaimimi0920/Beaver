"""Small independent control meshes; no character or external data."""
from copy import deepcopy
import unittest

from mesh_readback import compare


class MeshReadbackTests(unittest.TestCase):
    def setUp(self):
        self.mesh = {
            "vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
            "polygons": [[0, 1, 2, 3]],
            "triangles": [[0, 1, 2], [0, 2, 3]],
        }

    def test_author_and_render_edges_are_not_interchangeable(self):
        split = deepcopy(self.mesh)
        split["polygons"] = split["triangles"]
        report = compare(self.mesh, split, [1, 0, 0])
        self.assertEqual(report["control"]["native_triangles"], 2)
        self.assertEqual(report["control"]["author_edges"], 4)
        self.assertEqual(report["candidate"]["author_edges"], 5)
        self.assertEqual(report["added_author_edges"], [(0, 2)])
        self.assertTrue(report["boundary_coordinates_equal"])
        self.assertAlmostEqual(report["control"]["axis_edge_length_share"]["20"], 0.5)

    def test_identical_author_counts_do_not_hide_native_sliver(self):
        control = deepcopy(self.mesh)
        control["vertices"][1] = [0.01, 0, 0]
        candidate = deepcopy(control)
        candidate["triangles"] = [[0, 1, 3], [1, 2, 3]]
        report = compare(control, candidate)
        self.assertEqual(report["unchanged_ordered_author_faces"], 1)
        self.assertLess(report["candidate"]["minimum_native_triangle_quality"], 0.02)
        self.assertEqual(report["candidate"]["zero_quality_native_triangles"], 0)

    def test_same_boundary_connectivity_does_not_hide_vertex_motion(self):
        moved = deepcopy(self.mesh)
        moved["vertices"][0] = [0, 0, 0.01]
        report = compare(self.mesh, moved)
        self.assertTrue(report["boundary_indices_equal"])
        self.assertFalse(report["boundary_coordinates_equal"])
        self.assertEqual(report["changed_shared_vertex_indices"], 1)

    def test_invalid_native_triangle_count_is_not_silently_ignored(self):
        broken = deepcopy(self.mesh)
        broken["triangles"].pop()
        with self.assertRaisesRegex(ValueError, "native triangle count"):
            compare(self.mesh, broken)

    def test_inconsistent_winding_is_reported(self):
        split = deepcopy(self.mesh)
        split["polygons"] = split["triangles"] = [[0, 1, 2], [0, 3, 2]]
        self.assertEqual(
            compare(self.mesh, split)["candidate"]["inconsistent_interior_winding_edges"],
            [(0, 2)],
        )


if __name__ == "__main__":
    unittest.main()
