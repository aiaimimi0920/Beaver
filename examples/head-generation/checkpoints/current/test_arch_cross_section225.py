"""The authored cross-section bends internally without lifting its boundary rails."""
import unittest
from test_spatial_lashes196 import S, STYLE

class CrossSectionArchTests(unittest.TestCase):
    def compare(self, t, amount):
        lo = S['curve'](t, 1, STYLE['upper_lash_rails'])
        hi = S['curve'](t, 2, STYLE['upper_lash_rails'])
        rise = lo + (hi-lo)*amount
        saved = STYLE['accent_cross_arch_ratio']
        try:
            STYLE['accent_cross_arch_ratio'] = 0
            flat = S['upper_accent_point'](1, t, rise, 0)
            STYLE['accent_cross_arch_ratio'] = saved
            curved = S['upper_accent_point'](1, t, rise, 0)
            return flat, curved
        finally:
            STYLE['accent_cross_arch_ratio'] = saved

    def test_both_boundary_rails_stay_fixed(self):
        for t in [-.8, -.2, .2, .55, .86]:
            for amount in [0, 1]:
                a, b = self.compare(t, amount)
                for x, y in zip(a, b):
                    self.assertAlmostEqual(x, y, places=12)

    def test_crown_is_a_bounded_vertical_arch_not_whole_sheet_translation(self):
        for t in [-.2, .2, .55]:
            a, b = self.compare(t, .5)
            self.assertAlmostEqual(a[0], b[0], places=12)
            self.assertAlmostEqual(a[1], b[1], places=12)
            self.assertGreater(b[2]-a[2], .0005)
            self.assertLess(b[2]-a[2], .003)

    def test_root_to_free_edge_height_never_folds_back(self):
        for t in [-.8, -.2, .2, .55, .86]:
            lo = S['curve'](t, 1, STYLE['upper_lash_rails'])
            hi = S['curve'](t, 2, STYLE['upper_lash_rails'])
            heights = [S['upper_accent_point'](1, t, hi-(hi-lo)*i/100, 0)[2]
                       for i in range(101)]
            self.assertTrue(all(a >= b for a, b in zip(heights, heights[1:])))

if __name__ == '__main__':
    unittest.main()
