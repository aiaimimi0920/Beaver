"""Source-quad transport and protected full-patch transaction contracts."""
import unittest
import numpy as np
import mouth_patch_geometry as geometry
import mouth_patch_sampling as sampling
for name in ('mouth_patch_quad_weights','mouth_patch_feasible_step','mouth_patch_quad_quality'):
    setattr(sampling,name,getattr(geometry,name))


class MouthPatchTests(unittest.TestCase):
    def test_bilinear_inverse_and_shared_edge(self):
        vertices=np.array([(0,0,0),(1,0,0),(1.2,0,1),(0,0,1),(2,0,0),(2,0,1)],float)
        faces=[(0,1,2,3),(1,4,5,2)]
        weights=np.array([.21,.09,.21,.49])
        target=weights@vertices[[0,1,2,3]][:,[0,2]]
        result=geometry.mouth_patch_quad_weights(target,vertices,faces)
        self.assertEqual(len(result),1)
        np.testing.assert_allclose(result[0][1],weights,atol=1e-10)
        result=geometry.mouth_patch_quad_weights(np.array([1.1,.5]),vertices,faces)
        self.assertEqual(len(result),2)
        with self.assertRaises(ValueError):
            geometry.mouth_patch_quad_weights(np.array([3.,.5]),vertices,faces)
        with self.assertRaises(ValueError):
            geometry.mouth_patch_quad_weights(target,vertices,[faces[0],faces[0]])

    def test_first_convexity_boundary_is_uniform(self):
        old=np.array([(0,0,0),(1,0,0),(1,0,1),(0,0,1)],float)
        direction=np.array([(0,0),(-2,0),(-2,0),(0,0)],float)
        step,boundary=geometry.mouth_patch_feasible_step(old,direction,[(0,1,2,3)])
        self.assertAlmostEqual(boundary,.5)
        self.assertAlmostEqual(step,.45)
        candidate=old.copy();candidate[:,[0,2]]+=step*direction
        geometry.mouth_patch_quad_quality(candidate,[(0,1,2,3)])
        with self.assertRaises(ValueError):
            geometry.mouth_patch_quad_quality(old,[(0,2,1,3)])

    def test_capture_uses_current_rays_and_no_mutation(self):
        vertices=[(x,0,y) for x,y in [(0,0),(.1,0),(.05,.1),(.15,.1),
                                    (.1,.2),(.2,.2),(.15,.3),(.25,.3),(.2,.4),(.3,.4)]]
        rings=[list(range(2*r,2*r+2)) for r in range(5)]
        before=vertices.copy()
        result=geometry.capture_mouth_patch_fields(vertices,rings,[-1.,1.],1,
               {'upper_lip_edge_depth_delta':.001,'lower_lip_edge_depth_delta':.002})
        self.assertEqual(vertices,before)
        self.assertEqual(result['upper'],{1,3,5,7})
        self.assertEqual(set(result['rim']),set(range(8)))
        self.assertGreater(result['rim'][1],result['rim'][7])

    def test_invalid_contract_does_not_change_vertices(self):
        vertices=[(0,0,1.58),(.01,0,1.58)]
        before=vertices.copy()
        with self.assertRaises(ValueError):
            sampling.redistribute_mouth_patch(vertices,[],[],[],[],0,lambda x,y:(0,0))
        self.assertEqual(vertices,before)

    def test_packaged_recipe_keeps_helpers(self):
        from feature_flow_recipe import retained_sources
        sources=retained_sources()
        for name in ('mouth_patch_geometry.py','mouth_patch_sampling.py','source_mouth_patch_sampling.py'):
            self.assertIn(name,sources)


if __name__=='__main__':unittest.main()
