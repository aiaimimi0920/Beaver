"""Final outline check scope, using the real validator and an in-memory mesh."""
import math
import sys
import unittest
from types import SimpleNamespace as S
from unittest.mock import patch
from source_outline_stage import validate_final_outline_triangles


class Vector:
    def __init__(self, values):
        self.values = list(values)

    def __iter__(self):
        return iter(self.values)

    def __sub__(self, other):
        return Vector(a-b for a, b in zip(self.values, other.values))

    @property
    def x(self):
        return self.values[0]

    @x.setter
    def x(self, value):
        self.values[0] = value

    @property
    def y(self):
        return self.values[1]

    @property
    def length(self):
        return math.sqrt(sum(x*x for x in self.values))

    def normalize(self):
        length = self.length
        self.values = [x/length for x in self.values]

    def cross(self, other):
        a, b, c = self.values
        d, e, f = other.values
        return Vector((b*f-c*e, c*d-a*f, a*e-b*d))

    def dot(self, other):
        return sum(a*b for a, b in zip(self.values, other.values))


def mesh():
    points = [(1,0,0), (2,0,0), (1,0,1), (1,0,0), (2,0,0), (1,1,0)]
    attributes = {
        "OutlineOriginalFace": [1,2], "OutlineSourceRegion": [1,2],
        "NormalRegion": [1,2], "OutlineGeneratedReturn": [0,0],
    }
    return S(vertices=[S(co=Vector(p)) for p in points],
             polygons=[S(index=0, vertices=(0,1,2)), S(index=1, vertices=(3,4,5))],
             attributes={key:S(data=[S(value=v) for v in values]) for key, values in attributes.items()})


class OutlineValidationTests(unittest.TestCase):
    def setUp(self):
        context = patch.dict(sys.modules, {"mathutils": S(Vector=Vector)})
        context.start()
        self.addCleanup(context.stop)
        self.reference = {"affected_source_faces": [1], "reference_normals": {
            "1": [[0,-1,0]], "2": [[0,math.sqrt(1-.334**2),.334]]}}

    def test_unaffected_fitting_is_diagnostic_but_still_geometry_checked(self):
        result = validate_final_outline_triangles(mesh(), self.reference)
        self.assertEqual(result["finite_nondegenerate_source_triangles"], 2)
        self.assertEqual(result["triangles"], 1)
        self.assertAlmostEqual(result["unaffected_side_before_fit_diagnostics"][0]["normal_dot"], .334)

    def test_affected_direction_threshold_is_not_relaxed(self):
        reference = dict(self.reference, affected_source_faces=[1,2])
        with self.assertRaisesRegex(AssertionError, "changed original surface orientation"):
            validate_final_outline_triangles(mesh(), reference)

    def test_unaffected_degenerate_source_is_rejected(self):
        data = mesh()
        data.vertices[5].co = Vector((1,0,0))
        with self.assertRaisesRegex(AssertionError, "Invalid final source triangle"):
            validate_final_outline_triangles(data, self.reference)

    def test_missing_source_identity_is_rejected(self):
        data = mesh()
        data.attributes["OutlineOriginalFace"].data[1].value = 0
        with self.assertRaisesRegex(AssertionError, "Lost source face provenance"):
            validate_final_outline_triangles(data, self.reference)

    def test_generated_return_must_not_inherit_source_identity(self):
        data = mesh()
        data.attributes["OutlineGeneratedReturn"].data[1].value = 1
        with self.assertRaisesRegex(AssertionError, "Invalid generated return triangle"):
            validate_final_outline_triangles(data, self.reference)


if __name__ == "__main__":
    unittest.main()
