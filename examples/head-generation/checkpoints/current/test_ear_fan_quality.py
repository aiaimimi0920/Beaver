"""Malformed-fan CPU tests for the independent gate; not native proof."""
import math
import unittest

from ear_topology import source_fan_quality as check



class V(tuple):
    def __sub__(self, other):
        return V(a-b for a,b in zip(self,other))
    @property
    def length(self):
        return math.sqrt(self.dot(self))
    def dot(self, other):
        return sum(a*b for a,b in zip(self,other))


class Vertex:
    def __init__(self, point):
        self.co = V(point)
        self.link_faces = []


class Face:
    material_index = 0
    def __init__(self, vertices, region=47):
        self.verts = vertices
        self.region = region
        u,v = vertices[1].co-vertices[0].co,vertices[2].co-vertices[0].co
        n=V((u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]))
        self.area=n.length/2
        self.normal=V(x/n.length for x in n) if n.length else V((0,0,0))
        for vertex in vertices:
            vertex.link_faces.append(self)
    def __getitem__(self, key):
        return self.region
    def calc_area(self):
        return self.area


def fan(angles=(0,25,50,75,100,125),radius=.004):
    root=Vertex((0,0,0))
    ring=[Vertex((radius*math.cos(math.radians(a)),radius*math.sin(math.radians(a)),0)) for a in angles]
    faces=[Face([root,a,b]) for a,b in zip(ring,ring[1:])]
    return root,faces,ring


class FanQualityTests(unittest.TestCase):
    def test_good_fan_is_measured_without_modification(self):
        root,faces,ring=fan()
        before=[tuple(v.co) for v in [root]+ring]
        result=check(root,'region')
        self.assertIsNotNone(result)
        self.assertGreater(result['minimum_angle_degrees'],12)
        self.assertEqual(before,[tuple(v.co) for v in [root]+ring])
    def test_skinny_and_overwide_fans_rejected(self):
        for angles in [(0,1,50,75,100,125),(0,40,80,120,160,200)]:
            root,_,_=fan(angles)
            self.assertIsNone(check(root,'region'))
    def test_mixed_region_material_fold_and_duplicate_rejected(self):
        for defect in ['region','material','normal','duplicate','nan_normal','zero_normal']:
            root,faces,_=fan()
            if defect=='region':faces[0].region=48
            if defect=='material':faces[0].material_index=1
            if defect=='normal':faces[0].normal=V((0,0,-1))
            if defect=='duplicate':root.link_faces.append(faces[0])
            if defect=='nan_normal':faces[0].normal=V((float('nan'),0,1))
            if defect=='zero_normal':faces[0].normal=V((0,0,0))
            self.assertIsNone(check(root,'region'),defect)
    def test_nonfinite_and_out_of_radius_rejected(self):
        root,_,ring=fan()
        ring[0].co=V((float('nan'),0,0))
        self.assertIsNone(check(root,'region'))
        root,_,_=fan(radius=.013)
        self.assertIsNone(check(root,'region'))
    def test_disconnected_chain_rejected(self):
        root,faces,_=fan()
        root.link_faces.remove(faces[2])
        self.assertIsNone(check(root,'region'))




class FanSamplingTests(unittest.TestCase):
    def test_sampling_preserves_endpoints_and_equalizes_angles(self):
        import ast
        from pathlib import Path
        from types import SimpleNamespace
        tree=ast.parse(Path(__file__).with_name('ears.py').read_text())
        node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='lower_fan_stations')
        scope={'C':{},'H':1.,'Z0':0.,'sqrt':math.sqrt,'sin':math.sin,'pi':math.pi,
               'membrane_offset':lambda t,u,c,d:(.004*u,0.),
               'ear_arc_height':lambda t,low,high:.004}
        exec(compile(ast.Module(body=[node],type_ignores=[]),'sampling','exec'),scope)
        root=SimpleNamespace(x=0.,y=0.,z=.004)
        stations=scope['lower_fan_stations'](root,(0.,0.,0.),.1,0.,1.,1)
        self.assertEqual((stations[0],stations[-1]),(0.,1.))
        self.assertEqual(len(stations),6)
        self.assertTrue(all(b>a for a,b in zip(stations,stations[1:])))
        angles=[math.atan(u) for u in stations]
        for i,angle in enumerate(angles):
            self.assertAlmostEqual(angle,math.pi/4*i/5,places=4)
        scope['membrane_offset']=lambda t,u,c,d:(float('nan'),0.)
        with self.assertRaisesRegex(ValueError,'Nonfinite'):
            scope['lower_fan_stations'](root,(0.,0.,0.),.1,0.,1.,1)




class NeighborQualityTests(unittest.TestCase):
    def test_neighbor_cannot_become_skinny_or_nonfinite(self):
        from ear_topology import neighboring_quality_preserved
        vertices=[Vertex((0,0,0)),Vertex((.004,0,0)),Vertex((.002,.0035,0))]
        face=Face(vertices);old={v:V(v.co) for v in vertices}
        self.assertTrue(neighboring_quality_preserved([face],old))
        vertices[2].co=V((.002,.0001,0))
        self.assertFalse(neighboring_quality_preserved([face],old))
        vertices[2].co=V((float('nan'),0,0))
        self.assertFalse(neighboring_quality_preserved([face],old))
    def test_already_poor_neighbor_cannot_regress(self):
        from ear_topology import neighboring_quality_preserved
        vertices=[Vertex((0,0,0)),Vertex((.004,0,0)),Vertex((.002,.0001,0))]
        face=Face(vertices);old={v:V(v.co) for v in vertices}
        vertices[2].co=V((.002,.00005,0))
        self.assertFalse(neighboring_quality_preserved([face],old))
        vertices[2].co=V((.002,.0035,0))
        self.assertTrue(neighboring_quality_preserved([face],old))
    def test_policy_coverage_fails_before_touching_mesh(self):
        from ear_topology import reflow_source_fans
        self.assertTrue(reflow_source_fans(None,[])['not_applicable'])
        row={'source_fan_policy':'angular-source-fan-v1','root_positions':[(1,0,0)]}
        with self.assertRaises(AssertionError):reflow_source_fans(None,[row])
        with self.assertRaises(AssertionError):reflow_source_fans(None,[row,row])
    def test_resampling_preserves_endpoints(self):
        from ear_topology import resample_fan_chain
        points=[(.004*math.cos(math.radians(a)),.004*math.sin(math.radians(a)),0.) for a in (0,5,20,55,80,100)]
        out=resample_fan_chain((0,0,0),points)
        self.assertEqual((out[0],out[-1]),(points[0],points[-1]))
        for i,point in enumerate(out):
            self.assertAlmostEqual(math.degrees(math.atan2(point[1],point[0])),i*20,places=6)

if __name__=='__main__':
    unittest.main()
