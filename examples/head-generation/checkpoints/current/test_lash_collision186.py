"""Owner186: unchanged ocular surfaces must not cut through accent facets."""
import unittest
from pathlib import Path
from test_style_math import pure_function, STYLE

samples,_=pure_function('eye_accents.py','triangle_samples',{})
shift,_=pure_function('eye_accents.py','clearance_shift',{})
sections,_=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})
curve,_=pure_function('face.py','curve',{})
curve189,_=pure_function('face.py','curve',{})
outline189,_=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})
accent189,_=pure_function('eye_accents.py','accent_sections',{'STYLE':STYLE,'curve':curve189,'outline_sections':outline189})

class LashCollisionTests(unittest.TestCase):
    def test_curve_penetration_between_clear_vertices_is_detected(self):
        tri=[(-1,-.001,0),(1,-.001,0),(0,-.001,1)]
        def obstacle(x,z): return -.002*(1-x*x)*4*z*(1-z)
        self.assertTrue(all(shift(p,obstacle(p[0],p[2]),.0002)==0 for p in tri))
        moves=[shift(p,obstacle(p[0],p[2]),.0002) for p in samples(*tri)]
        self.assertLess(min(moves),-.001)
    def test_clearance_moves_only_forward_and_preserves_missing_support(self):
        self.assertEqual(shift((0,0,0),None,.001),0)
        self.assertEqual(shift((0,-.004,0),-.001,.001),0)
        self.assertAlmostEqual(shift((0,0,0),-.001,.00035),-.00135)
    def test_inner_fork_has_distinct_tips_and_open_notch(self):
        main=accent189('upper_lash_rails');fork=sections(STYLE['inner_lash_fork_outline'])
        self.assertLess(min(p[0] for p in fork),min(p[0] for p in main))  # 481: lower branch extends farther inward
        self.assertLess(min(p[0] for p in fork),max(p[0] for p in fork)-.3)
        t=-.85  # both branches exist here; do not test extrapolated values
        self.assertGreater(curve(t,1,main)-curve(t,2,fork),.003)
    def test_fitter_includes_ocular_obstacles_and_triangle_samples(self):
        source=Path(__file__).with_name('eye_accents.py').read_text()
        self.assertIn("obj.get('ocular_surface')",source)
        self.assertIn('for sample in triangle_samples(*points)',source)
        self.assertIn("minimum_sampled_obstacle_clearance_m",source)

if __name__=='__main__':unittest.main()
