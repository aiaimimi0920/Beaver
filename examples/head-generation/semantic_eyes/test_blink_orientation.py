"""CPU-only mesh doubles exercise the continuous pose gate without Blender."""

import math
from types import SimpleNamespace
import unittest

from blink_runtime import verify_pose_orientation


class Vector:
    def __init__(self, values):
        self.values = tuple(values)

    def copy(self):
        return Vector(self.values)

    def __add__(self, other):
        return Vector(a + b for a, b in zip(self.values, other.values))

    def __sub__(self, other):
        return Vector(a - b for a, b in zip(self.values, other.values))

    def dot(self, other):
        return sum(a * b for a, b in zip(self.values, other.values))

    def cross(self, other):
        x, y, z = self.values
        a, b, c = other.values
        return Vector((y * c - z * b, z * a - x * c, x * b - y * a))

    @property
    def length_squared(self):
        return self.dot(self)

    @property
    def length(self):
        return math.sqrt(self.length_squared)


def mesh(basis=None, *, left=None, right=None, mouth=None, polygons=None):
    basis = basis if basis is not None else [(0, 0, 0), (1, 0, 0), (0, 1, 0)]
    neutral = [Vector(v) for v in basis]
    blocks = {"Basis": SimpleNamespace(data=[SimpleNamespace(co=v) for v in neutral])}
    for name, movement in [("blink.L", left), ("blink.R", right), ("MouthOpen", mouth)]:
        movement = movement if movement is not None else [(0, 0, 0)] * len(basis)
        blocks[name] = SimpleNamespace(
            data=[SimpleNamespace(co=v + Vector(d)) for v, d in zip(neutral, movement)]
        )
    faces = polygons if polygons is not None else [(0, 1, 2)]
    return SimpleNamespace(
        data=SimpleNamespace(
            shape_keys=SimpleNamespace(key_blocks=blocks),
            polygons=[
                SimpleNamespace(vertices=ids, index=i) for i, ids in enumerate(faces)
            ],
        )
    )


class BlinkOrientationTests(unittest.TestCase):
    def test_neutral_triangle(self):
        report = verify_pose_orientation(mesh())
        self.assertEqual(report["triangles"], 1)
        self.assertEqual(report["minimum_orientation_ratio"], 1)

    def test_additive_blink_and_mouth(self):
        report = verify_pose_orientation(
            mesh(
                left=[(0, 0, 0), (-0.25, 0, 0), (0, 0, 0)],
                mouth=[(0, 0, 0), (0, 0, 0), (0, -0.5, 0)],
            )
        )
        self.assertAlmostEqual(report["minimum_orientation_ratio"], 0.375)
        self.assertEqual(report["worst_blink_weight"], 1)
        self.assertEqual(report["worst_mouth_weight"], 1)

    def test_independent_eyes_on_separate_triangles(self):
        report = verify_pose_orientation(
            mesh(
                basis=[
                    (0, 0, 0),
                    (1, 0, 0),
                    (0, 1, 0),
                    (2, 0, 0),
                    (3, 0, 0),
                    (2, 1, 0),
                ],
                left=[(0, 0, 0), (-0.25, 0, 0)] + [(0, 0, 0)] * 4,
                right=[(0, 0, 0)] * 5 + [(0, -0.5, 0)],
                polygons=[(0, 1, 2), (3, 4, 5)],
            )
        )
        self.assertEqual(report["triangles"], 2)
        self.assertAlmostEqual(report["minimum_orientation_ratio"], 0.5)
        self.assertEqual(report["worst_triangle"], 1)

    def test_failure_identifies_triangle_and_exact_pose(self):
        with self.assertRaises(AssertionError) as caught:
            verify_pose_orientation(mesh(left=[(0, 0, 0), (-2, 0, 0), (0, 0, 0)]))
        message = str(caught.exception)
        for detail in (
            "triangle=0",
            "vertices=[0, 1, 2]",
            "orientation_ratio=-1",
            "blink=1",
            "mouth=0",
        ):
            self.assertIn(detail, message)

    def test_reversed_triangle(self):
        with self.assertRaisesRegex(AssertionError, "folds a triangle"):
            verify_pose_orientation(mesh(left=[(0, 0, 0), (-2, 0, 0), (0, 0, 0)]))

    def test_interior_collapse_with_positive_endpoints(self):
        # Orientation is (1 - 2 * blink)^2: endpoints alone would both pass.
        with self.assertRaisesRegex(AssertionError, "folds a triangle"):
            verify_pose_orientation(mesh(left=[(0, 0, 0), (-2, 0, 0), (0, -2, 0)]))

    def test_degenerate_neutral_triangle(self):
        with self.assertRaisesRegex(AssertionError, "Degenerate neutral triangle"):
            verify_pose_orientation(mesh(basis=[(0, 0, 0), (1, 0, 0), (2, 0, 0)]))

    def test_nonfinite_morph_coordinates(self):
        for value in (float("nan"), float("inf"), -float("inf")):
            with self.subTest(value=value):
                with self.assertRaisesRegex(
                    AssertionError, "Nonfinite pose coefficients"
                ):
                    verify_pose_orientation(
                        mesh(mouth=[(0, 0, 0), (value, 0, 0), (0, 0, 0)])
                    )

    def test_cross_eye_triangle(self):
        with self.assertRaisesRegex(AssertionError, "couples both eye controls"):
            verify_pose_orientation(
                mesh(
                    left=[(0, 0, 0), (-0.1, 0, 0), (0, 0, 0)],
                    right=[(0, 0, 0), (0, 0, 0), (0, -0.1, 0)],
                )
            )

    def test_nontriangular_polygon(self):
        with self.assertRaisesRegex(AssertionError, "requires actual triangles"):
            verify_pose_orientation(mesh(polygons=[(0, 1, 2, 0)]))


if __name__ == "__main__":
    unittest.main()
