"""Continuous source-domain guards for joint cheek chart redistribution."""
import unittest
from source_feature_flow import cheek_source_transition, cheek_lower_transition


class CheekTransitionTests(unittest.TestCase):
    def test_lower_field_keeps_mouth_and_outer_boundaries(self):
        for h in (1.570, 1.599, 1.6045, 1.614):
            for q in (.44, .55, .67, .78, .87, .94):
                self.assertEqual(cheek_lower_transition(h,q,q,h,1), (q,h))
        for h in (1.580, 1.590):
            for q in (.18, .44, .94, 1.):
                self.assertEqual(cheek_lower_transition(h,q,q,h,1), (q,h))
        x, y = cheek_lower_transition(1.580,.55,.5,1.58,1)
        self.assertAlmostEqual(x-.5,-.007846401962650448)
        self.assertAlmostEqual(y-1.58,-.00082888564544505)

    def test_lower_field_is_c1_and_disabled_exactly(self):
        epsilon = 1e-8
        for h, inside in ((1.570, epsilon), (1.599, -epsilon)):
            x, y = cheek_lower_transition(h+inside, .67, .67, h+inside, 1)
            self.assertLess(abs(x-.67)/epsilon, .0001)
            self.assertLess(abs(y-h-inside)/epsilon, .0001)
        for q, inside in ((.44, epsilon), (.94, -epsilon)):
            x, y = cheek_lower_transition(1.590, q+inside, q+inside, 1.590, 1)
            self.assertLess(abs(x-q-inside)/epsilon, .0001)
            self.assertLess(abs(y-1.590)/epsilon, .0001)
        self.assertEqual(cheek_lower_transition(1.590, .67, .65, 1.591, 0), (.65, 1.591))

    def test_protected_boundaries_and_disabled_field_are_exact(self):
        for h in (1.570, 1.599, 1.614, 1.673):
            for q in (.0, .18, .25, .44, .67, .78, 1.):
                self.assertEqual(cheek_source_transition(h, q, q+.01, h+.001, 1), (q+.01, h+.001))
        for q in (.0, .18, .78, 1.):
            self.assertEqual(cheek_source_transition(1.6045, q, q, 1.6045, 1), (q, 1.6045))
        for q in (.25, .34, .44, .55, .67):
            self.assertEqual(cheek_source_transition(1.6045, q, q, 1.6045, 0), (q, 1.6045))

    def test_semantic_center_station_and_linear_strength(self):
        q, y = cheek_source_transition(1.6045, .44, .4, 1.605, 1)
        self.assertAlmostEqual(q-.4, -.009999999999999993)
        self.assertAlmostEqual(y-1.605, -.0006926238636225346)
        half = cheek_source_transition(1.6045, .44, .4, 1.605, .5)
        self.assertAlmostEqual(half[0], (.4+q)/2)
        self.assertAlmostEqual(half[1], (1.605+y)/2)

    def test_boundary_correction_has_zero_first_derivative(self):
        epsilon = 1e-8
        for h, inside in ((1.599, epsilon), (1.614, -epsilon)):
            q, y = cheek_source_transition(h+inside, .44, .44, h+inside, 1)
            self.assertLess(abs(q-.44)/epsilon, .0001)
            self.assertLess(abs(y-h-inside)/epsilon, .0001)
        for q, inside in ((.18, epsilon), (.78, -epsilon)):
            x, y = cheek_source_transition(1.6045, q+inside, q+inside, 1.6045, 1)
            self.assertLess(abs(x-q-inside)/epsilon, .0001)
            self.assertLess(abs(y-1.6045)/epsilon, .0001)


if __name__ == '__main__':
    unittest.main()
