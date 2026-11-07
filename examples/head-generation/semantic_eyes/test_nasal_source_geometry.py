"""Current source-surface identity and index-independent replacement domain."""
import unittest
import numpy as np
from nasal_source_geometry import AuthorQuadSurface, canonical_boundary, clipped_polygon, polygon_area
from nasal_source_domain import select_source_domain


class NasalSourceGeometryTests(unittest.TestCase):
    def test_author_quad_and_fixed_edge_are_the_same_surface(self):
        vertices=np.array([(0,0,0),(1,-.1,0),(1,-.4,1),(0,-.2,1)],float)
        surface=AuthorQuadSurface(vertices,[(0,1,2,3)],[1])
        np.testing.assert_allclose(surface.sample(.5,.5),(.5,-.175,.5))
        for t in np.linspace(0,1,11):
            np.testing.assert_allclose(surface.sample(t,0),vertices[0]*(1-t)+vertices[1]*t)
        ids,weights,point=surface.lineage(.25,.75)
        np.testing.assert_allclose(np.asarray(weights)@vertices[list(ids)],point)
        with self.assertRaises(ValueError):surface.sample(1.1,.5)
        with self.assertRaises(ValueError):AuthorQuadSurface(vertices,[],[])

    def test_overlap_and_shared_edge_are_distinguished(self):
        vertices=np.array([(0,0,0),(1,0,0),(1,0,1),(0,0,1),(2,0,0),(2,0,1)],float)
        surface=AuthorQuadSurface(vertices,[(0,1,2,3),(1,4,5,2)],[1,1])
        np.testing.assert_allclose(surface.sample(1,.5),(1,0,.5))
        doubled=AuthorQuadSurface(vertices,[(0,1,2,3),(0,1,2,3)],[1,1])
        with self.assertRaises(ValueError):doubled.sample(.5,.5)

    def fixture(self):
        vertices=np.array([(x*.01,0,1.6+y*.01) for y in range(3) for x in range(3)])
        faces=[(y*3+x,y*3+x+1,(y+1)*3+x+1,(y+1)*3+x) for y in range(2) for x in range(2)]
        return vertices,faces

    def test_canonical_boundary_is_equivariant_to_reindexing(self):
        vertices,faces=self.fixture()
        loop=canonical_boundary(vertices,faces,set(range(4)))
        self.assertEqual(loop[0],0)
        self.assertEqual(loop[1],1)
        permutation=np.array([4,8,1,5,0,7,3,6,2])
        reordered=np.empty_like(vertices);reordered[permutation]=vertices
        mapped=[tuple(int(permutation[i]) for i in f) for f in faces]
        next_loop=canonical_boundary(reordered,mapped,set(range(4)))
        np.testing.assert_array_equal(reordered[next_loop],vertices[loop])

    def test_single_pass_domain_and_feature_refusal(self):
        vertices,faces=self.fixture();before=vertices.copy()
        cell=np.array([[.002,1.602],[.018,1.602],[.018,1.618],[.002,1.618]])
        core=np.column_stack((cell[:,0],np.zeros(4),cell[:,1]))
        axis={0,3,6};features={'protected':axis,'axis':axis}
        chosen,boundary,report=select_source_domain(vertices,faces,[1]*4,[cell],core,features)
        self.assertEqual(chosen,set(range(4)));self.assertEqual(len(boundary),8)
        self.assertEqual(report['notch_faces'],0)
        np.testing.assert_array_equal(vertices,before)
        with self.assertRaises(ValueError):
            select_source_domain(vertices,faces,[1]*4,[cell],core,{'protected':axis|{4},'axis':axis})

    def test_clipping_handles_orientation_without_moving_inputs(self):
        a=np.array([(0,0),(2,0),(2,2),(0,2)],float)
        b=np.array([(1,1),(3,1),(3,3),(1,3)],float)
        self.assertAlmostEqual(polygon_area(clipped_polygon(a,b)),1)
        self.assertAlmostEqual(polygon_area(clipped_polygon(a,b[::-1])),1)


if __name__=='__main__':unittest.main()
