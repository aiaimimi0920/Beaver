"""CPU geometry/UV tests for the real final-source fairing implementation."""
import math
import unittest
from types import SimpleNamespace as S
from source_triangle_fairing import fair_final_source_triangles


def fixture():
    points = []
    for row in range(9):
        for column in range(9):
            x = (column-4)*.01
            z = 1.50+row*.01
            y = .007*math.exp(-((z-1.54)/.008)**2)*math.exp(-x*x/.002)
            points.append((x,y,z))
    faces = []
    for row in range(8):
        for col in range(8):
            a = row*9+col
            faces.extend([(a,a+1,a+10),(a,a+10,a+9)])
    project = lambda x,z: (.5+x/.235,(z-1.5)/.31)
    loops = [S(index=k,vertex_index=i) for k,i in enumerate(i for face in faces for i in face)]
    uv2 = S(data=[S(uv=project(points[l.vertex_index][0],points[l.vertex_index][2])) for l in loops])
    material = S(data=[S(uv=(.3,.4)) for _ in loops])
    data = S(vertices=[S(co=p) for p in points],polygons=[S(vertices=f) for f in faces],
             attributes={'NormalRegion':S(data=[S(value=1) for _ in faces])},edges=[],loops=loops,
             uv_layers={'UV2':uv2,'UVMap':material},update=lambda:None)
    config = {'nose_stations':[(0,0,0),(.01,0,0)],'mouth_height':2.,'mouth_half_width':.01}
    return data, config, points, faces, project


class TriangleFairingTests(unittest.TestCase):
    def test_actual_geometry_improves_without_topology_growth(self):
        data, config, points, faces, project = fixture()
        result = fair_final_source_triangles(data,config,.1,1.5,0.,project_uv=project)
        self.assertEqual([p.vertices for p in data.polygons],faces)
        self.assertEqual(len(data.vertices),len(points))
        self.assertGreater(result['moved_vertices'],0)
        self.assertLess(result['adjacency_energy_after'],result['adjacency_energy_before'])
        self.assertLess(result['maximum_patch_dihedral_after_degrees'],result['maximum_patch_dihedral_before_degrees'])
        self.assertLessEqual(result['maximum_displacement_m'],.0015+result['displacement_roundoff_tolerance_m'])
        self.assertTrue(result['protected_features_unchanged'])
        for row in range(9):
            for column in range(9):
                i = row*9+column
                if row in (0,1,7,8) or column in (0,1,4,7,8):
                    self.assertEqual(tuple(data.vertices[i].co),points[i])
                j = row*9+8-column
                a,b = data.vertices[i].co,data.vertices[j].co
                self.assertAlmostEqual(a[0],-b[0],places=7)
                self.assertAlmostEqual(a[1],b[1],places=7)
                self.assertAlmostEqual(a[2],b[2],places=7)
        self.assertGreater(result['uv2_reprojected_corners'],0)
        for loop in data.loops:
            point = data.vertices[loop.vertex_index].co
            self.assertEqual(data.uv_layers['UV2'].data[loop.index].uv,project(point[0],point[2]))
            self.assertEqual(data.uv_layers['UVMap'].data[loop.index].uv,(.3,.4))

    def test_repeat_is_deterministic(self):
        outputs = []
        for _ in range(2):
            data, config, _, _, project = fixture()
            report = fair_final_source_triangles(data,config,.1,1.5,0.,project_uv=project)
            outputs.append(([tuple(v.co) for v in data.vertices],report))
        self.assertEqual(outputs[0],outputs[1])

    def test_no_eligible_skin_stays_unchanged(self):
        data, config, points, _, project = fixture()
        for item in data.attributes['NormalRegion'].data:
            item.value = 8
        report = fair_final_source_triangles(data,config,.1,1.5,0.,project_uv=project)
        self.assertEqual(report['moved_vertices'],0)
        self.assertEqual([tuple(v.co) for v in data.vertices],points)

    def test_invalid_budget_or_mesh_rejected(self):
        for budget in (0,-1,float('nan')):
            data, config, _, _, project = fixture()
            with self.assertRaises(AssertionError):
                fair_final_source_triangles(data,config,.1,1.5,0.,maximum_move=budget,project_uv=project)
        data, config, _, _, project = fixture()
        data.vertices[0].co = (float('nan'),0,0)
        with self.assertRaises(AssertionError):
            fair_final_source_triangles(data,config,.1,1.5,0.,project_uv=project)


if __name__ == '__main__':
    unittest.main()
