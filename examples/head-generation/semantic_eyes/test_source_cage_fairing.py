"""Mirror seam and rollback invariants for coarse source placement."""
import sys,types,unittest,math
from unittest.mock import patch
from source_cage_fairing import fair_source_cage

class V:
    def __init__(self,values=(0.,0.,0.)):self.v=list(values)
    def __iter__(self):return iter(self.v)
    def __add__(self,o):return V(a+b for a,b in zip(self,o))
    def __sub__(self,o):return V(a-b for a,b in zip(self,o))
    def __mul__(self,s):return V(x*s for x in self)
    __rmul__=__mul__
    def __truediv__(self,s):return self*(1/s)
    def copy(self):return V(self.v)
    @property
    def x(self):return self.v[0]
    @x.setter
    def x(self,x):self.v[0]=x
    @property
    def length(self):return math.sqrt(sum(x*x for x in self))
    def dot(self,o):return sum(a*b for a,b in zip(self,o))

class Data:
    def __init__(self,feature=False,bad=False):
        S=types.SimpleNamespace
        self.vertices=[S(co=V(p)) for p in [(0,0,0),(.01,0,0),(.01,.01,0),(0,.01,0)]]
        self.edges=[S(index=i,vertices=e) for i,e in enumerate([(0,1),(1,2),(2,3),(3,0)])]
        self.polygons=[S(vertices=(0,1,2,3),normal=V((0,0,1)),area=.0001)]
        self.attributes={'crease_edge':S(data=[S(value=0) for _ in self.edges]),'FacialCreaseGroup':S(data=[S(value=3 if feature else 0) for _ in self.edges])}
        self.bad=bad
    def update(self):
        if self.bad:self.polygons[0].normal=V((0,0,-1))

class CageTests(unittest.TestCase):
    def run_fairing(self,data):
        with patch.dict(sys.modules,{'mathutils':types.SimpleNamespace(Vector=V)}):
            return fair_source_cage(types.SimpleNamespace(data=data))
    def test_mirror_plane_exact_and_topology_fixed(self):
        data=Data();before=[tuple(v.co) for v in data.vertices];faces=[p.vertices for p in data.polygons]
        report=self.run_fairing(data)
        self.assertEqual(data.vertices[0].co.x,0);self.assertEqual(data.vertices[3].co.x,0)
        self.assertEqual(faces,[p.vertices for p in data.polygons]);self.assertEqual(len(data.vertices),4)
        self.assertLessEqual(report['maximum_move_m'],.002+1e-12)
        self.assertNotEqual(before,[tuple(v.co) for v in data.vertices])
    def test_feature_neighborhood_does_not_move(self):
        data=Data(feature=True);before=[tuple(v.co) for v in data.vertices];self.run_fairing(data)
        self.assertEqual(before,[tuple(v.co) for v in data.vertices])
    def test_failed_orientation_restores_positions(self):
        data=Data(bad=True);before=[tuple(v.co) for v in data.vertices]
        with self.assertRaises(ValueError):self.run_fairing(data)
        self.assertEqual(before,[tuple(v.co) for v in data.vertices])

if __name__=='__main__':unittest.main()
