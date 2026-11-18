"""Small source-relative donor tests, not native or artistic acceptance."""
import unittest
import numpy as np
from nasal_regularization_chart import ConnectedQuadChart


class ConnectedNasalChartTests(unittest.TestCase):
    def setUp(self):
        self.vertices = np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]],dtype=float)
        self.chart = ConnectedQuadChart(self.vertices, [[0,1,2,3]], [1])

    def test_identity_is_exact(self):
        r = self.chart.sample_from_vertex(0, np.array([0.,0.]))
        np.testing.assert_array_equal(r['point'],self.vertices[0])
        self.assertEqual(tuple(r['donors']),(0,))

    def test_bilinear_lineage(self):
        r = self.chart.sample_from_vertex(0,np.array([.25,.75]))
        np.testing.assert_allclose(r['point'],[.25,0,.75],atol=1e-12)
        self.assertAlmostEqual(sum(r['weights']),1.)
        self.assertGreaterEqual(min(r['weights']),0.)

    def test_overlapping_interior_is_not_nearest_depth(self):
        vertices = np.r_[self.vertices,self.vertices[1:]]
        chart = ConnectedQuadChart(vertices,[[0,1,2,3],[0,4,5,6]],[1,1])
        with self.assertRaisesRegex(ValueError,'Multiple interior'):
            chart.sample_from_vertex(0,np.array([.2,.2]))

    def test_leaving_connected_domain_fails(self):
        with self.assertRaises(ValueError):
            self.chart.sample_from_vertex(0,np.array([2.,.2]))

    def test_reindexing_preserves_point_and_weights(self):
        permutation = np.array([2,0,3,1])
        inverse = np.argsort(permutation)
        chart = ConnectedQuadChart(self.vertices[permutation],
                                   [inverse[[0,1,2,3]].tolist()],[1])
        a = self.chart.sample_from_vertex(0,np.array([.2,.6]))
        b = chart.sample_from_vertex(int(inverse[0]),np.array([.2,.6]))
        np.testing.assert_allclose(a['point'],b['point'],atol=1e-12)
        self.assertEqual([int(permutation[i]) for i in b['donors']],list(a['donors']))


if __name__ == '__main__':
    unittest.main()
