"""CPU contract stubs match Blender's index-based tessellator, not native proof."""

import math
import sys
import types
import unittest
from unittest.mock import patch

import brow_mesh
from brow_design import validate
from brow_shapes import brow_polygons, brow_area, brow_cross, brow_sampling_divisions


class Vector(tuple):
    def __new__(cls, values):
        return tuple.__new__(cls, values)

    def __sub__(self, other):
        return Vector(a - b for a, b in zip(self, other))

    def dot(self, other):
        return sum(a * b for a, b in zip(self, other))

    def cross(self, b):
        a = self
        return Vector((a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]))

    @property
    def length(self):
        return math.sqrt(self.dot(self))


class NativeContractTests(unittest.TestCase):
    def setUp(self):
        self.objects = []
        self.face = types.ModuleType("face")
        self.face.H, self.face.Y0 = 1.0, 1.5
        self.face.C = {"brow_inner_x": .02, "brow_outer_x": .08, "brow_height": 0, "brow_rise": 0, "brow_thickness": .002}
        self.face.STYLE = {"brow_rgb": (.1, .1, .1)}
        self.face.depth = lambda x, y: .1 - 20*x*x - 10*(y-1.5)**2
        self.face.coord = lambda x, y, z: (x, -z, y)
        def mesh(name, vertices, faces, color):
            obj = {"name": name, "vertices": vertices, "faces": faces}
            self.objects.append(obj)
            return obj
        self.face.mesh = mesh
        self.patch = patch.dict(sys.modules, {"face": self.face, "mathutils": types.SimpleNamespace(Vector=Vector), "mathutils.geometry": types.SimpleNamespace(tessellate_polygon=lambda polygons: [(0, 1, 2)])})
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.helpers = patch.multiple(brow_mesh, brow_polygons=brow_polygons, brow_area=brow_area, brow_cross=brow_cross, brow_sampling_divisions=brow_sampling_divisions, create=True)
        self.helpers.start()
        self.addCleanup(self.helpers.stop)
        brow_mesh.BROW_GEOMETRY_REPORT.clear()

    def test_real_indices_closed_islands_and_mirror_orientation(self):
        d = validate({"schema_version": 1, "kind": "symbols", "detail_count": 3})
        for side in (-1, 1):
            brow_mesh.build_designed_brow(side, d)
        self.assertEqual([o["name"] for o in self.objects], ["Tapered brow -1", "Tapered brow 1"])
        for row in brow_mesh.BROW_GEOMETRY_REPORT:
            self.assertEqual(row["islands"], 3)
            self.assertEqual(row["triangles"], 2*row["vertices"]-12)
            self.assertGreater(row["signed_volume_m3"], 0)
            self.assertGreater(row["sampled_min_depth_clearance_m"], 0)
        left, right = self.objects
        self.assertEqual(len(left["vertices"]), len(right["vertices"]))
        for a, b in zip(left["vertices"], right["vertices"]):
            self.assertAlmostEqual(a[0], -b[0])
            self.assertEqual(a[1:], b[1:])

    def test_cap_refinement_rejects_nominal_clearance_as_measurement(self):
        d = validate({"schema_version": 1, "kind": "symbols", "detail_count": 2, "width_scale": 3, "clearance_m": .0003, "thickness_m": .0002})
        polygon = brow_polygons(d, .06, .002)[0]
        vertices = brow_mesh.brow_surface_points(polygon, 1, d)
        original, _ = brow_mesh.brow_sample_clearance(vertices, [(0, 1, 2)])
        self.assertLess(original, 0)
        brow_mesh.build_designed_brow(1, d)
        report = brow_mesh.BROW_GEOMETRY_REPORT[0]
        self.assertGreater(max(report["cap_refinement_rounds"]), 0)
        self.assertGreaterEqual(report["sampled_min_depth_clearance_m"], .00005)
        self.assertNotAlmostEqual(report["sampled_min_depth_clearance_m"], report["requested_vertex_depth_clearance_m"], places=8)


if __name__ == "__main__":
    unittest.main()
