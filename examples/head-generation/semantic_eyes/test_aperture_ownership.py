"""Closed-loop ownership rejects coordinate-only false eye boundaries."""
import unittest

from source_aperture_ownership import closed_interior_boundary_edges


class ApertureOwnershipTests(unittest.TestCase):
    def setUp(self):
        self.points = [(0, 0, 0), (4, 0, 0), (4, 4, 0), (0, 4, 0),
                       (1, 1, 0), (3, 1, 0), (3, 3, 0), (1, 3, 0)]
        self.faces = [(0, 1, 5, 4), (1, 2, 6, 5),
                      (2, 3, 7, 6), (3, 0, 4, 7)]
        self.eye = {(4, 5), (5, 6), (6, 7), (4, 7)}

    def test_closed_eye_only(self):
        self.assertEqual(closed_interior_boundary_edges(self.faces, self.points), self.eye)

    def test_outer_eye_height_does_not_create_another_aperture(self):
        points = [(x * .02, y, 1.64) for x, y, _ in self.points]
        self.assertEqual(closed_interior_boundary_edges(self.faces, points), self.eye)

    def test_face_order_does_not_change_ownership(self):
        self.assertEqual(closed_interior_boundary_edges(self.faces[::-1], self.points), self.eye)

    def test_reversed_winding_does_not_change_ownership(self):
        faces = [tuple(reversed(f)) for f in self.faces]
        self.assertEqual(closed_interior_boundary_edges(faces, self.points), self.eye)

    def test_missing_closed_eye_is_rejected(self):
        with self.assertRaises(ValueError):
            closed_interior_boundary_edges(self.faces[:-1], self.points)

    def test_nonmanifold_is_rejected(self):
        with self.assertRaises(ValueError):
            closed_interior_boundary_edges(self.faces + [self.faces[0]] * 2, self.points)

    def test_extra_positive_boundary_is_ambiguous(self):
        points = [(x + 1, y, z) for x, y, z in self.points]
        with self.assertRaises(ValueError):
            closed_interior_boundary_edges(self.faces, points)

    def test_repeated_face_vertex_rejected(self):
        with self.assertRaises(ValueError):
            closed_interior_boundary_edges(self.faces + [(0, 1, 1)], self.points)


if __name__ == '__main__':
    unittest.main()
