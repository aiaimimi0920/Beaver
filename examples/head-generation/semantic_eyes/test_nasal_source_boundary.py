"""Shared-point donor snapshots and source feature exclusion regression tests."""
import unittest
from types import SimpleNamespace
import numpy as np
from nasal_source_boundary import (snapshot_nasal_fields, apply_nasal_fields,
                                   boundary_aperture_distances, boundary_patch_metrics)


class Layer:
    def __init__(self,name): self.name=name


class Layers:
    def __init__(self,name): self.name=name
    def values(self): return [Layer(self.name)]


class Vertex:
    def __init__(self,value,vector,flag,deform):
        self.fields={'value':value,'vector':vector,'flag':flag,'weights':deform}
    def __getitem__(self,layer): return self.fields[layer.name]
    def __setitem__(self,layer,value): self.fields[layer.name]=value


class NasalSourceBoundaryTests(unittest.TestCase):
    def fixture(self):
        bm=SimpleNamespace(verts=SimpleNamespace(layers=SimpleNamespace(
            float=Layers('value'),float_vector=Layers('vector'),int=Layers('flag'),
            deform=Layers('weights'))))
        parents=[Vertex(1.,[1.,2.,3.],0,{0:1.}),Vertex(3.,[3.,4.,5.],0,{1:1.})]
        return bm,parents

    def test_snapshot_is_independent_of_write_order_and_wrapper_identity(self):
        bm,parents=self.fixture()
        moves={0:{'lineage':((0,1),(.25,.75))},1:{'lineage':((0,1),(.5,.5))}}
        snapshots=snapshot_nasal_fields(bm,parents,moves)
        parents[0].fields['weights'][99]=5.
        apply_nasal_fields(parents[0],snapshots[0])
        apply_nasal_fields(parents[1],snapshots[1])
        self.assertEqual(parents[0].fields['value'],2.5)
        self.assertEqual(parents[1].fields['value'],2.)
        self.assertEqual(parents[1].fields['vector'],(2.,3.,4.))
        self.assertEqual(parents[0].fields['weights'],{0:.25,1:.75})
        self.assertEqual(parents[1].fields['weights'],{0:.5,1:.5})

    def test_invalid_weights_and_discrete_seams_are_rejected(self):
        bm,parents=self.fixture()
        with self.assertRaises(ValueError):
            snapshot_nasal_fields(bm,parents,{0:{'lineage':((0,1),(.7,.7))}})
        parents[1].fields['flag']=1
        with self.assertRaisesRegex(ValueError,'discrete'):
            snapshot_nasal_fields(bm,parents,{0:{'lineage':((0,1),(.5,.5))}})

    def test_aperture_graph_distance_does_not_confuse_second_and_first_lid(self):
        graph={i:set() for i in range(6)}
        for i in range(5): graph[i].add(i+1);graph[i+1].add(i)
        d=boundary_aperture_distances({'apertures':{0},'skin_neighbors':graph})
        self.assertEqual(d,{0:0,1:1,2:2,3:3,4:4})

    def test_patch_quality_extrema_and_degenerate_edge(self):
        points=np.array([(0,0,0),(2,0,0),(2,0,1),(0,0,1)],float)
        m=boundary_patch_metrics(points,[(0,1,2,3)])
        self.assertEqual(m,{'min_angle':90.,'max_angle':90.,'max_edge_ratio':2.})
        points[1]=points[0]
        with self.assertRaises(ValueError): boundary_patch_metrics(points,[(0,1,2,3)])


if __name__=='__main__':unittest.main()
