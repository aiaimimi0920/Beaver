"""Pure source-flow contracts; native geometry fixtures remain separate evidence."""
import unittest
import numpy as np
from forehead_flow_rule import (forehead_chart, forehead_convex_chart,
                                forehead_no_chart_crossings, propose_forehead_flow, forehead_open_boundary_guard)
from forehead_flow_bind import bind_forehead_paths, mirror_forehead_paths
from feature_flow_recipe import retained_sources
from forehead_flow_refine import refine_forehead_terminal


class ForeheadFlowTests(unittest.TestCase):
    def test_chart_preserves_height_and_left_right_symmetry(self):
        points = np.array([[.03, -.08, 1.7], [-.03, -.08, 1.7], [0, -.1, 1.8]])
        chart = forehead_chart(points)
        self.assertAlmostEqual(chart[0, 0], -chart[1, 0])
        np.testing.assert_array_equal(chart[:, 2], points[:, 2])
        np.testing.assert_array_equal(chart[:, 1], 0)

    def test_invalid_radius_rejected(self):
        with self.assertRaises(ValueError):
            forehead_chart(np.zeros((4, 3)))

    def test_chart_quad_orientation(self):
        p = np.array([[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]], float)
        self.assertTrue(np.all(forehead_convex_chart(p, [[0, 1, 2, 3]]) > 0))
        with self.assertRaises(ValueError):
            forehead_convex_chart(p, [[0, 2, 1, 3]])

    def test_geometric_crossing_is_not_only_shared_ids(self):
        p = np.array([[0,0,0], [1,0,1], [0,0,1], [1,0,0]], float)
        with self.assertRaises(ValueError):
            forehead_no_chart_crossings(p, [[0,1], [2,3]])
        forehead_no_chart_crossings(p, [[0,1], [1,2]])

    def test_five_distinct_paths_required(self):
        with self.assertRaises(ValueError):
            propose_forehead_flow(np.zeros((10,3)), [], [[0,1,2,3,4]]*5, set(), 0.)

    def test_binding_rejects_unversioned_chart(self):
        with self.assertRaises(ValueError):
            bind_forehead_paths([[0,0,0]], [], [], {'semantic_rows': [0,1]})

    def test_mirror_rejects_ambiguous_points(self):
        points = [[1,0,1], [-1,0,1], [-1,0,1]]
        with self.assertRaises(ValueError):
            mirror_forehead_paths(points, [], [], {'positive_paths': {'red':[0]}, 'pins':[0]})

    def test_mirror_owner_pairs_must_be_involutive(self):
        with self.assertRaises(ValueError):
            mirror_forehead_paths([[1,0,1]], [], [], {'positive_paths':{},'pins':[]}, {1:2})

    def test_existing_open_boundary_is_allowed_but_cannot_move(self):
        points = np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]],float)
        proof = forehead_open_boundary_guard(points,points.copy(),[[0,1,2,3]],[0])
        self.assertEqual(len(proof['existing_open_edges']),4)
        changed=points.copy();changed[0,0]=.01
        with self.assertRaises(ValueError):
            forehead_open_boundary_guard(points,changed,[[0,1,2,3]],[0])

    def test_nonmanifold_source_not_grandfathered(self):
        points=np.zeros((8,3))
        with self.assertRaises(ValueError):
            forehead_open_boundary_guard(points,points,[[0,1,2,3],[1,0,4,5],[0,1,6,7]],[0])

    def test_terminal_solver_rejects_incomplete_paths(self):
        with self.assertRaises(ValueError):
            refine_forehead_terminal(np.zeros((20,3)), [], [[0,1]]*5,
                                     {'vertices':np.zeros((20,3))})

    def test_terminal_solver_cannot_release_pinned_roles(self):
        paths=[list(range(i*20,i*20+19)) for i in range(5)]
        with self.assertRaises(ValueError):
            refine_forehead_terminal(np.zeros((100,3)), [], paths,
                                     {'vertices':np.zeros((100,3)), 'lineage':{}})

    def test_all_flow_modules_are_retained_readable_source(self):
        sources = retained_sources()
        for name in ('forehead_flow_bind.py','forehead_flow_rule.py','forehead_flow_refine.py','forehead_flow_native.py'):
            self.assertIn(name, sources)
            compile(sources[name], name, 'exec')
        self.assertIn('forehead_context', sources['source_nasal_spacing.py'])


if __name__ == '__main__':
    unittest.main()
