import unittest
from test_spatial_lashes196 import S, STYLE

class LateralSurfaceTests(unittest.TestCase):
    def test_width_stations_follow_their_own_skin_depth(self):
        for side in (-1, 1):
            f = 1.0
            x0, y = S['lateral_position'](side, f, 0.0)
            x1, _ = S['lateral_position'](side, f, 0.008)
            p0 = S['lateral_accent_point'](side, f, 0.0, 0.0)
            p1 = S['lateral_accent_point'](side, f, 0.008, 0.0)
            expected = -(S['depth'](x1, y)-S['depth'](x0, y))*(1-STYLE['accent_lateral_depth_blend'])
            self.assertAlmostEqual(p1[1]-p0[1], expected, places=10)
            self.assertGreater(abs(expected), 0.00001)
