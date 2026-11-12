"""Boundary facts never depend on radial coordinates or whole-shell tags."""
import unittest
from construction_boundaries import boundary_facts


class BoundaryRoleTests(unittest.TestCase):
    def test_nonzero_fraction_station_on_outline_is_not_interior(self):
        points = ["seam_low", "q_one_twelfth_low", "outer_low", "seam_high", "q_one_twelfth_high", "outer_high"]
        faces = [[points[i] for i in row] for row in ([0, 1, 4, 3], [1, 2, 5, 4])]
        result = boundary_facts(faces, points, declared_seam=[points[0], points[3]])
        self.assertIn("q_one_twelfth_high", result["open_boundary_except_mirror_seam"])
        self.assertIn("q_one_twelfth_low", result["open_boundary_except_mirror_seam"])
        self.assertEqual(sum(row["kind"] == "declared_half_mirror_seam" for row in result["actual_boundary_edges"]), 1)

    def test_aperture_requires_actual_open_edges_not_a_tag(self):
        faces = [["a", "b", "c", "d"]]
        result = boundary_facts(faces, list("abcd"), aperture_paths={"eye_aperture": {"point_ids": list("abcd"), "closed": True}})
        self.assertTrue(all(row["kind"] == "eye_aperture" for row in result["actual_boundary_edges"]))
        with self.assertRaisesRegex(ValueError, "actual open boundary"):
            boundary_facts(faces, list("abcd"), aperture_paths={"fake": {"point_ids": ["a", "c"], "closed": False}})

    def test_joined_mirror_center_edge_is_not_an_outline(self):
        faces = [["a", "b", "c", "d"], ["d", "e", "f", "a"]]
        result = boundary_facts(faces, list("abcdef"))
        edges = {frozenset(row["point_ids"]) for row in result["actual_boundary_edges"]}
        self.assertNotIn(frozenset(("a", "d")), edges)


if __name__ == "__main__":
    unittest.main()
