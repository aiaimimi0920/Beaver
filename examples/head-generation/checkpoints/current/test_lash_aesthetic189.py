"""Semantic shape checks; not a substitute for the owner's aesthetic review."""
import unittest
from test_style_math import pure_function, STYLE
curve,_=pure_function('face.py','curve',{})
sections,_=pure_function('eye_accents.py','accent_sections',{'STYLE':STYLE,'curve':curve})

class LashAestheticTests(unittest.TestCase):
    def test_main_rails_have_pointed_ends_and_positive_interior(self):
        rows=sections('upper_lash_rails')
        self.assertAlmostEqual(rows[0][1],rows[0][2])
        self.assertAlmostEqual(rows[-1][1],rows[-1][2])
        self.assertTrue(all(hi>lo for t,lo,hi in rows[1:-1]))
        self.assertTrue(all(b[0]-a[0] <= STYLE['accent_max_station_gap']+1e-12 for a,b in zip(rows,rows[1:])))
    def test_main_sweep_has_continuous_tangents_away_from_tips(self):
        rails=STYLE['upper_lash_rails']
        for x,*_ in rails[1:-1]:
            for axis in (1,2):
                h=1e-5
                l=(curve(x,axis,rails)-curve(x-h,axis,rails))/h
                r=(curve(x+h,axis,rails)-curve(x,axis,rails))/h
                self.assertLess(abs(l-r),.0001)
    def test_fork_and_tuft_are_subordinate_swept_shapes(self):
        fork=STYLE['inner_lash_fork_outline']
        self.assertGreater(min(p[1] for p in fork),-.01)  # 245: inner fork descends slightly below corner height
        self.assertLess(min(p[1] for p in fork),.005)
        self.assertGreater(max(p[1] for p in fork),.025)
        tuft=STYLE['upper_swept_tuft_outline']
        tip=max(tuft,key=lambda p:p[1])
        self.assertGreater(tip[0],.2)
        self.assertLess(tip[0],.65)  # coarse reference comparison: tip sits farther outward
        self.assertLess(tip[1],.08)
    def test_lateral_primary_dominates_shorter_companion(self):
        self.assertGreater(STYLE['calibration']['outer_liner_width'],2.5*STYLE['accent_lateral_branch_width_H'])
        self.assertEqual(STYLE['accent_lateral_branch_start'],.04) # Near-root start matches the reference strip
        self.assertEqual(STYLE['accent_lateral_branch_span'],.92)
        self.assertLess(STYLE['accent_lateral_branch_start']+STYLE['accent_lateral_branch_span'],1)
        self.assertLess(STYLE['accent_lateral_bottom_t'],.8)
