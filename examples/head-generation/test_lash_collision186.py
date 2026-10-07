"""Owner186: unchanged ocular surfaces must not cut through accent facets."""
import unittest
from pathlib import Path
from test_style_math import pure_function, STYLE

samples,_=pure_function('eye_accents.py','triangle_samples',{})
shift,_=pure_function('eye_accents.py','clearance_shift',{})
sections,_=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})
curve,_=pure_function('face.py','curve',{})

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
        main=sections(STYLE['upper_lash_outline']);fork=sections(STYLE['inner_lash_fork_outline'])
        self.assertLess(min(p[0] for p in fork),min(p[0] for p in main))
        t=-1.05
        self.assertGreater(curve(t,1,main)-curve(t,2,fork),.005)
    def test_fitter_includes_ocular_obstacles_and_triangle_samples(self):
        source=Path(__file__).with_name('eye_accents.py').read_text()
        self.assertIn("obj.get('ocular_surface')",source)
        self.assertIn('for sample in triangle_samples(*points)',source)
        self.assertIn("minimum_sampled_obstacle_clearance_m",source)

if __name__=='__main__':unittest.main()
