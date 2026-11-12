import unittest
import numpy as np
from closest_surface import closest


class ClosestSurfaceTests(unittest.TestCase):
    def test_inside_and_outside_triangle(self):
        vertices = np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        faces = np.array([[0, 1, 2]])
        results = closest(
            np.array([[0.25, 0.25, 2.0], [2.0, 0.0, 0.0]]), vertices, faces
        )
        self.assertAlmostEqual(results[0][0], 2.0)
        np.testing.assert_allclose(results[0][2], [0.5, 0.25, 0.25])
        self.assertAlmostEqual(results[1][0], 1.0)
        np.testing.assert_allclose(results[1][2], [0.0, 1.0, 0.0])

    def test_empty_samples(self):
        vertices = np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        self.assertEqual(closest(np.empty((0, 3)), vertices, np.array([[0, 1, 2]])), [])


if __name__ == "__main__":
    unittest.main()
