"""Source-chart invariants, not native mesh or artistic acceptance."""
import unittest
from source_feature_flow import feature_flow_chart


class FeatureFlowTests(unittest.TestCase):
    def test_fixed_outer_rim_and_inactive_ranges(self):
        for i in range(201):
            y = 1.53 + .26*i/200
            self.assertEqual(feature_flow_chart(y, 1)[0], 1)
            if y <= 1.580 or y >= 1.608:
                self.assertEqual(feature_flow_chart(y, 1), (1, y))
            for q in (0, .18, .44, .67, 1):
                self.assertEqual(feature_flow_chart(y, q, 0), (q, y))
                if y <= 1.551 or y >= 1.710:
                    self.assertEqual(feature_flow_chart(y, q), (q, y))

    def test_cheek_guide_has_no_outward_turnback(self):
        for height in (1.599, 1.614):
            heights = [feature_flow_chart(height, i/100)[1] for i in range(101)]
            self.assertTrue(all(b <= a + 1e-12 for a,b in zip(heights,heights[1:])))

    def test_landmark_flow_directions(self):
        self.assertLess(feature_flow_chart(1.673, 0)[1], feature_flow_chart(1.673, .55)[1])
        self.assertGreater(feature_flow_chart(1.599, 0)[1], feature_flow_chart(1.599, .9)[1])
        self.assertLess(feature_flow_chart(1.595, .67)[0], feature_flow_chart(1.673, .67)[0])
        self.assertLess(feature_flow_chart(1.605, .25)[0], feature_flow_chart(1.673, .25)[0])
        for y in (1.570, 1.599, 1.614, 1.673):
            self.assertEqual(feature_flow_chart(y, 0)[0], 0)

    def test_positive_chart_jacobian_and_row_column_order(self):
        epsilon = 1e-6
        smallest = 1.0
        for j in range(1, 100):
            y = 1.551 + .159*j/100
            previous = -1
            for i in range(1, 100):
                q = i/100
                x, z = feature_flow_chart(y, q)
                self.assertGreater(x, previous)
                previous = x
                a, b = feature_flow_chart(y, q + epsilon)
                c, d = feature_flow_chart(y + epsilon, q)
                determinant = ((a-x)*(d-z) - (c-x)*(b-z))/(epsilon*epsilon)
                smallest = min(smallest, determinant)
                self.assertGreater(determinant, .15)
        self.assertGreater(smallest, .15)


if __name__ == '__main__':
    unittest.main()
