"""Tests for the authored sheet pitch; visual validation remains separate."""
import unittest
from pathlib import Path
from test_style_math import STYLE

class FoldDirectionTests(unittest.TestCase):
    def test_upper_margin_projects_forward_from_attached_lower_rail(self):
        slope=STYLE['accent_sheet_slope']
        self.assertGreater(slope,.1)
        self.assertLess(slope,1.6)
        for t,lo,hi in STYLE['upper_lash_rails']:
            projection=(hi-lo)*STYLE['calibration']['height_m']*slope
            self.assertGreaterEqual(projection,0)
            self.assertLess(projection,.014)
    def test_builder_uses_shared_main_rail_support_and_keeps_obstacle_guard(self):
        text=Path(__file__).with_name('eye_accents.py').read_text()
        self.assertIn('upper_attached_depth(x,anchor_y,thickness)',text)
        self.assertIn('max(0.0,anchor-rise)*H*STYLE["accent_sheet_slope"]',text)
        self.assertIn("obj.get('ocular_surface')",text)
        self.assertIn('for sample in triangle_samples(*points)',text)

    def test_ridge_and_free_tuft_are_not_solid_wall_thickness(self):
        self.assertLess(STYLE['accent_band_thickness_m'],.0003)
        self.assertGreater(STYLE['accent_band_roll_m'],3*STYLE['accent_band_thickness_m'])
        self.assertLess(STYLE['accent_tuft_flare_m'],.018)  # bounded free tip, attached root remains fixed
