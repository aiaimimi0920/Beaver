"""Full lower-guide supports are eligible; functional/non-skin boundaries stay fixed."""
import unittest
import numpy as np
from forehead_fit_domain import prepare_forehead_fit_domain, close_forehead_quality_domain
from forehead_fit_density import forehead_constraint_scaling, forehead_constraint_layout
from forehead_star_topology import forehead_edge_counts


class FitDomainTests(unittest.TestCase):
    def test_fixed_bad_corner_requires_its_determining_support(self):
        points = np.array([[.01,0,0],[.02,0,0],[.03,0,.001],[.02,0,.01]])
        moving, affected, added = close_forehead_quality_domain(
            points, [[0,1,2,3]], {3}, {1,2}, {0})
        self.assertIn(0,moving)
        self.assertEqual(added,[0])
        self.assertEqual(affected,{0})
        with self.assertRaises(ValueError):
            close_forehead_quality_domain(points,[[0,1,2,3]],{3},{0,1,2},{0})

    def test_corner_constraint_layout_keeps_every_angle_bound(self):
        layout = forehead_constraint_layout(2,3,1)
        self.assertEqual(layout['angle_lower'],(0,8))
        self.assertEqual(layout['angle_upper'],(8,16))
        self.assertEqual(layout['aspect'],(16,18))
        self.assertEqual(layout['orientation'],(18,26))
        self.assertEqual(layout['field'],(26,29))
        self.assertEqual(layout['lower_turn'],(29,30))
        self.assertEqual(layout['count'],30)
        for angles in (np.full((2,4),90.),np.array([[151.,151.,30.,28.],[90.,90.,90.,90.]])):
            compressed = np.r_[angles.min(axis=1)-25.003,149.997-angles.max(axis=1)]
            separate = np.r_[(angles-25.003).ravel(),(149.997-angles).ravel()]
            self.assertEqual(bool(np.all(compressed>=0)),bool(np.all(separate>=0)))

    def test_constraint_scaling_preserves_the_feasible_set(self):
        constraints = lambda p: np.array([p[0]-3,1000*p[1]+2,7.])
        scales = forehead_constraint_scaling(constraints,lambda p: True,np.zeros(2))
        np.testing.assert_allclose(scales,[1,1000,1])
        for point in ([0,0],[4,0],[4,-.003],[3,-.002]):
            raw = constraints(point)
            np.testing.assert_array_equal(raw >= 0,raw/scales >= 0)
        backward = forehead_constraint_scaling(lambda p: p,
            lambda p: None if p[0] > 0 else True,np.zeros(1))
        np.testing.assert_allclose(backward,[1])

    def test_lower_neighbors_are_not_frozen_by_height(self):
        width = 7
        points = np.array([[x*.01,-.1,1.5+z*.01] for z in range(7) for x in range(width)])
        faces = [[z*width+x,z*width+x+1,(z+1)*width+x+1,(z+1)*width+x]
                 for z in range(6) for x in range(6)]
        regions = [1]*len(faces)
        regions[0] = 47
        protected = {i for e,n in forehead_edge_counts(faces).items() if n==1 for i in e}
        protected.add(2*width+4)
        path = list(range(3*width,4*width))
        proposal = {'vertices':points.copy(),'faces':faces,'protected':sorted(protected),
                    'paths':{'blue':path},'moving':path[1:-1], 'affected_old':[], 'affected_new':[],
                    'topology':{'face_owners':list(range(len(faces)))}}
        result = prepare_forehead_fit_domain(points,faces,regions,proposal)
        self.assertIn(2*width+3,result['moving'])
        self.assertLess(points[2*width+3,2],1.595)
        self.assertNotIn(2*width+4,result['moving'])
        self.assertTrue(set(faces[0]).isdisjoint(result['moving']))
        np.testing.assert_array_equal(result['vertices'],points)
        with self.assertRaises(ValueError):
            prepare_forehead_fit_domain(points,faces,regions,proposal,rings=3)

        # An opposite-oriented return sheet remains frozen rather than becoming
        # an ambiguous second source for nearby frontal finite differences.
        faces[8] = list(reversed(faces[8]))
        proposal.update(faces=faces,affected_old=[],affected_new=[],moving=path[1:-1],
                        protected=sorted(protected))
        result = prepare_forehead_fit_domain(points,faces,regions,proposal)
        self.assertIn(8,result['support_domain']['opposite_chart_sheet_faces_frozen'])
        self.assertTrue(set(faces[8]).isdisjoint(result['moving']))
        self.assertNotIn(faces[8],result['source_faces'])


if __name__ == '__main__':
    unittest.main()
