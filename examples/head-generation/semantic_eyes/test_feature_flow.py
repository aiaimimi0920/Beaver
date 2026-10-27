"""Source-chart invariants, not native mesh or artistic acceptance."""
import unittest
from source_feature_flow import feature_flow_chart, orbital_shell_rounding, mouth_shell_rounding


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
        for height in (1.599,):
            heights = [feature_flow_chart(height, i/100)[1] for i in range(101)]
            self.assertTrue(all(b <= a + 1e-12 for a,b in zip(heights,heights[1:])))

    def test_eye_frame_rounding_is_compact_and_preserves_reference_domains(self):
        # The 1.614 eye-frame boundary now curves locally around the eye instead
        # of being constrained to a globally horizontal/monotone rectangle.
        # The nasal cheek guide above still has the original no-turnback test.
        for y in (1.599, 1.608, 1.686):
            for q in (0, .12, .18, .56, .94, 1):
                self.assertEqual(orbital_shell_rounding(y,q,q,y,1),(q,y))
        for y in (1.614, 1.6435, 1.673):
            for q in (0, 1):
                self.assertEqual(orbital_shell_rounding(y,q,q,y,1),(q,y))
        for y in (1.614, 1.673):
            for q in (.18, .94):
                x,z = orbital_shell_rounding(y,q,q,y,1)
                self.assertLess(abs(x-.56),abs(q-.56))
                self.assertLess(abs(z-1.6435),abs(y-1.6435))
                self.assertLessEqual(abs(x-q),.01521)
                self.assertLessEqual(abs(z-y),.001181)

    def test_mouth_outer_rounding_preserves_protected_domains(self):
        for y in (1.55,1.56,1.608,1.614,1.673):
            for q in (0,.18,.34,.44,.55,.67,1):
                self.assertEqual(mouth_shell_rounding(y,q,q,y,1),(q,y))
        for y in (1.570,1.580,1.590,1.599):
            for q in (0,.12,.18,.22,1):
                self.assertEqual(mouth_shell_rounding(y,q,q,y,1),(q,y))
            for q in (.34,.44,.55,.67):
                self.assertEqual(mouth_shell_rounding(y,q,q,y,0),(q,y))
        for y in (1.570,1.599):
            q,z=mouth_shell_rounding(y,.44,.44,y,1)
            self.assertLess(q,.44)
            self.assertLess(abs(z-1.5845),abs(y-1.5845))
        for q in (.55,.67):
            self.assertLess(mouth_shell_rounding(1.599,q,q,1.599,1)[0],q)

    def test_landmark_flow_directions(self):
        self.assertLess(feature_flow_chart(1.673, 0)[1], feature_flow_chart(1.673, .55)[1])
        self.assertGreater(feature_flow_chart(1.599, 0)[1], feature_flow_chart(1.599, .9)[1])
        self.assertLess(feature_flow_chart(1.595, .67)[0], feature_flow_chart(1.673, .67)[0])
        self.assertLess(feature_flow_chart(1.605, .25)[0], feature_flow_chart(1.673, .25)[0])
        for y in (1.570, 1.599, 1.614, 1.673):
            self.assertEqual(feature_flow_chart(y, 0)[0], 0)

    def test_upper_cheek_fade_reaches_next_retained_row_only(self):
        # Continue rounding into the next real grid row, not eye/mouth interiors.
        for q in (.25, .34, .44, .55, .67):
            x, y = mouth_shell_rounding(1.6045, q, q, 1.6045, 1)
            self.assertLess(x, q)
            self.assertLess(y, 1.6045)
        for h in (1.608, 1.614):
            for q in (.25, .44, .67):
                self.assertEqual(mouth_shell_rounding(h, q, q, h, 1), (q, h))
        for h in (1.570, 1.599):
            q = .44
            nx, ny = q/.44, (h-1.5845)/.0145
            from source_feature_flow import flow_smoothstep
            old_window = (flow_smoothstep((nx-.5)/.5)*(1-flow_smoothstep((nx-1)/.65))
                          *(1-flow_smoothstep((abs(ny)-1)/.25)))
            expected = (q-.12*old_window*.44*nx*ny*ny, h-.12*old_window*.0145*ny*nx*nx)
            self.assertEqual(mouth_shell_rounding(h, q, q, h, 1), expected)

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
