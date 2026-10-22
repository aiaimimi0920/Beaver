"""Pure topology probes with a deterministic analytic-support test double."""

import json
import math
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch
from nose_mesh import pad_geometry, faceted_geometry, orient_and_check
from nose_mesh import evaluated_skin_support, validate_final_attachments


class V(tuple):
    def __new__(cls, value):
        return super().__new__(cls, value)

    def copy(self):
        return V(self)

    def __sub__(self, o):
        return V(a - b for a, b in zip(self, o))

    def dot(self, o):
        return sum(a * b for a, b in zip(self, o))

    def cross(self, o):
        return V(
            (
                self[1] * o[2] - self[2] * o[1],
                self[2] * o[0] - self[0] * o[2],
                self[0] * o[1] - self[1] * o[0],
            )
        )

    @property
    def length(self):
        return math.sqrt(self.dot(self))


class NoseMeshTests(unittest.TestCase):
    def setUp(self):
        self.presets = json.loads((Path(__file__).parent / "presets.json").read_text())
        self.face = types.SimpleNamespace(
            H=0.236,
            Y0=1.544,
            depth=lambda x, y: 0.09 + 0.2 * x * x,
            coord=lambda x, y, z: (x, -z, y),
        )

    def test_closed_oriented_special_topology(self):
        with patch.dict(
            sys.modules,
            {"face": self.face, "mathutils": types.SimpleNamespace(Vector=V)},
        ):
            for fn, key in [
                (pad_geometry, "pad-muzzle"),
                (faceted_geometry, "faceted-candidate"),
            ]:
                v, f, volume = fn(self.presets[key])
                self.assertGreater(volume, 0)
                self.assertEqual(len(f), 2 * len(v) - 4)
                directions = {}
                for tri in f:
                    for a, b in zip(tri, tri[1:] + tri[:1]):
                        directions[(a, b)] = directions.get((a, b), 0) + 1
                self.assertTrue(
                    all(
                        n == 1 and directions.get((b, a)) == 1
                        for (a, b), n in directions.items()
                    )
                )

    def test_one_flipped_face_is_rejected(self):
        vertices = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)]
        faces = [(0, 2, 1), (0, 1, 3), (1, 2, 3), (2, 0, 3)]
        with patch.dict(sys.modules, {"mathutils": types.SimpleNamespace(Vector=V)}):
            orient_and_check(vertices, faces)
            faces[0] = faces[0][::-1]
            with self.assertRaises(ValueError):
                orient_and_check(vertices, faces)

    def test_evaluated_support_cleanup_and_depth_sign(self):
        calls = []

        class Matrix:
            def __matmul__(self, value):
                return V(value)

        class Tree:
            def ray_cast(self, origin, direction, distance):
                calls.append(tuple(origin))
                return types.SimpleNamespace(y=-0.12), None, 0, 0.88

        class Factory:
            @staticmethod
            def FromPolygons(points, polygons, all_triangles):
                self.assertFalse(all_triangles)
                self.assertEqual(points, [V((0, 0, 0))])
                return Tree()

        evaluated = types.SimpleNamespace(
            matrix_world=Matrix(),
            to_mesh=lambda: types.SimpleNamespace(
                vertices=[types.SimpleNamespace(co=V((0, 0, 0)))],
                polygons=[types.SimpleNamespace(vertices=(0, 0, 0))],
            ),
            to_mesh_clear=lambda: calls.append("cleared"),
        )
        shell = types.SimpleNamespace(evaluated_get=lambda graph: evaluated)
        bpy = types.SimpleNamespace(
            context=types.SimpleNamespace(evaluated_depsgraph_get=lambda: object())
        )
        with patch.dict(
            sys.modules,
            {
                "bpy": bpy,
                "mathutils": types.SimpleNamespace(Vector=V),
                "mathutils.bvhtree": types.SimpleNamespace(BVHTree=Factory),
            },
        ):
            support = evaluated_skin_support(shell)
            self.assertEqual(calls, ["cleared"])
            self.assertEqual(support(0.01, 1.61), 0.12)
            self.assertEqual(calls[-1], (0.01, -1.0, 1.61))

    def test_final_sample_assembly_translation(self):
        calls = []

        class Tree:
            def ray_cast(self, origin, direction, distance):
                calls.append(tuple(origin))
                return types.SimpleNamespace(y=-0.10), None, 0, 0.90

        factory = types.SimpleNamespace(FromPolygons=lambda *a, **kw: Tree())
        data = types.SimpleNamespace(
            vertices=[types.SimpleNamespace(co=V((0, 0, 0)))],
            polygons=[types.SimpleNamespace(vertices=(0, 0, 0), index=0)],
            attributes={
                "NormalRegion": types.SimpleNamespace(
                    data=[types.SimpleNamespace(value=1)]
                )
            },
        )
        caps = [{"name": "nose", "points": [(0.01, -0.099, 1.61)]}]
        with patch.dict(
            sys.modules,
            {
                "mathutils": types.SimpleNamespace(Vector=V),
                "mathutils.bvhtree": types.SimpleNamespace(BVHTree=factory),
            },
        ), patch("nose_mesh.NOSE_BACK_CAPS", caps):
            checks = validate_final_attachments(data)
            self.assertAlmostEqual(calls[0][2], 1.589)
            self.assertAlmostEqual(checks[0]["min_back_cap_embedding_m"], 0.001)
            caps[0]["points"] = [(0.01, -0.102, 1.61)]
            with self.assertRaises(ValueError):
                validate_final_attachments(data)

    def test_open_mesh_rejected(self):
        with patch.dict(
            sys.modules, {"mathutils": types.SimpleNamespace(Vector=V)}
        ), self.assertRaises(ValueError):
            orient_and_check([(0, 0, 0), (1, 0, 0), (0, 1, 0)], [(0, 1, 2)])


if __name__ == "__main__":
    unittest.main()
