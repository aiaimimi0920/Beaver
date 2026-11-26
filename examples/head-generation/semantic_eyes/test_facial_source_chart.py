import copy
import unittest
import numpy as np
from facial_source_chart import facial_source_chart
from facial_quad_transport import (facial_quad_weights, facial_quad_transport,
                                   facial_transport_candidates)


def ring():
    vertices = np.array([[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0],
                         [-1,-1,.2],[1,-1,.2],[1,1,.2],[-1,1,.2]],float)
    quads = [[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7]]
    triangles = [t for a,b,c,d in quads for t in ([a,b,c],[a,c,d])]
    return vertices,quads,triangles,[0,0,1,1,2,2,3,3],[[0,1,2,3],[4,7,6,5]]


class ChartTests(unittest.TestCase):
    def test_annulus_caps_removed_immutable(self):
        args=ring();before=copy.deepcopy(args);result=facial_source_chart(*args)
        self.assertEqual(result['original_ids'],list(range(8)))
        self.assertEqual(len(result['triangles']),8)
        self.assertEqual(result['certificate']['virtual_cap_triangles'],4)
        self.assertFalse(result['certificate']['virtual_caps_exported'])
        self.assertGreater(result['certificate']['minimum_signed_double_area'],0)
        for a,b in zip(args,before):np.testing.assert_array_equal(a,b)

    def test_permuted_original_ids(self):
        v,q,t,o,b=ring();ids=np.array([71,3,62,99,4,20,11,45]);r=facial_source_chart(
            v,ids[q],ids[t],o,[ids[l] for l in b],original_ids=ids)
        self.assertEqual(r['original_ids'],ids.tolist())
        np.testing.assert_allclose(r['uv'],facial_source_chart(v,q,t,o,b)['uv'])

    def test_partition_winding_and_missing_reject(self):
        for mutation in ('winding','missing','boundary','owner'):
            v,q,t,o,b=ring()
            if mutation=='winding':t[0]=t[0][::-1]
            if mutation=='missing':t=t[:-1];o=o[:-1]
            if mutation=='boundary':b[1]=b[1][::-1]
            if mutation=='owner':o[0]=99
            with self.assertRaises(ValueError):facial_source_chart(v,q,t,o,b)

    def test_disconnected_domain_rejected(self):
        vertices = np.array([[0,0,0],[1,0,0],[1,1,0],[0,1,0],
                             [3,0,0],[4,0,0],[4,1,0],[3,1,0]], float)
        with self.assertRaises(ValueError):
            facial_source_chart(vertices, [[0,1,2,3],[4,5,6,7]],
                [[0,1,2],[0,2,3],[4,5,6],[4,6,7]], [0,0,1,1],
                [[0,1,2,3],[4,5,6,7]])

    def test_nonmanifold_shared_edge_rejected(self):
        vertices = np.array([[0,0,0],[1,0,0],[1,1,0],[0,1,0],
                             [0,-1,0],[1,-1,0],[1,0,1],[0,0,1]], float)
        quads = [[0,1,2,3],[1,0,4,5],[0,1,6,7]]
        triangles = [t for a,b,c,d in quads for t in ([a,b,c],[a,c,d])]
        with self.assertRaisesRegex(ValueError, 'Nonmanifold'):
            facial_source_chart(vertices, quads, triangles, [0,0,1,1,2,2], [])

    def test_flipped_quad_partition_rejected(self):
        vertices, quads, triangles, owners, boundaries = ring()
        quads[0] = quads[0][::-1]
        with self.assertRaisesRegex(ValueError, 'partition'):
            facial_source_chart(vertices, quads, triangles, owners, boundaries)

    def test_invalid_ids_and_source(self):
        v,q,t,o,b=ring()
        with self.assertRaises(ValueError):facial_source_chart(v,q,t,o,b,original_ids=[0]*8)
        v[0,0]=np.nan
        with self.assertRaises(ValueError):facial_source_chart(v,q,t,o,b)


class TransportTests(unittest.TestCase):
    def test_bilinear_not_planar(self):
        q=np.array([[0,0,0],[1,0,0],[1,1,1],[0,1,0]],float)
        value,w=facial_quad_transport(q,[[0,0],[1,0],[1,1]],[.5,0,.5])
        np.testing.assert_allclose(w,[.25]*4)
        np.testing.assert_allclose(value,[.5,.5,.25])

    def test_diagonal_continuity(self):
        rng=np.random.default_rng(7)
        for u in rng.random(30):
            a=facial_quad_weights([[0,0],[1,0],[1,1]],[1-u,0,u])
            b=facial_quad_weights([[0,0],[1,1],[0,1]],[1-u,u,0])
            np.testing.assert_allclose(a,b,rtol=0,atol=0)

    def test_ambiguity_not_average(self):
        with self.assertRaises(ValueError):facial_transport_candidates([np.array([0.]),np.array([1.])])
        with self.assertRaises(ValueError):facial_transport_candidates([np.array([1])])
        with self.assertRaises(ValueError):facial_quad_transport(np.arange(4),[[0,0],[1,0],[1,1]],[1,0,0])
        with self.assertRaises(ValueError):facial_quad_weights([[0,0],[1,0],[1,1]],[-.1,.5,.6])
        x=np.array([1.]);y=facial_transport_candidates([x,x.copy()]);y[0]=2;self.assertEqual(x[0],1)

if __name__=='__main__':unittest.main()
