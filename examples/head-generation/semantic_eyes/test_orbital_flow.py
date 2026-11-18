"""Checks for outside-aperture continuity and bounded source ring sampling."""
import math
import unittest
import source_orbital_continuity as orbital
from source_eye_ring_flow import eye_source_ring_fraction

class OrbitalFlowTest(unittest.TestCase):
    def setUp(self):
        orbital.eye_warp = lambda t,c:t
        orbital.lower_coordinate = lambda t,c:t
        self.c={'eye_upper_arch':.09,'eye_lower_arch':.06,'eye_upper_power':1,
                'eye_lower_power':1,'eye_horizontal_power':2.24,
                'eye_lower_horizontal_power':2.3,'orbital_lateral_fade':.16,'orbital_band_width':.28}
        self.H=.24

    def test_aperture_and_external_region_unchanged(self):
        for t,dy in [(0,.0001),(1,0),(-1,0),(1.1,.005)]:
            self.assertEqual(orbital.orbital_continuous_weight(t,dy,.4,self.c,self.H),.4)

    def test_bridge_bounds_and_endpoint_slope(self):
        for t in [1.01,1.05,1.1,1.15,-1.05]:
            h=.002*(abs(t)-1)/.16
            def w(y):
                original=orbital.orbital_endpoint_weight(t,y,y>=0,self.c,self.H)[0]
                return orbital.orbital_continuous_weight(t,y,original,self.c,self.H)
            eps=1e-9
            for side in [-1,1]:
                y=side*h
                target,slope=orbital.orbital_endpoint_weight(t,y,side>0,self.c,self.H)
                self.assertAlmostEqual(w(y),target,places=12)
                self.assertAlmostEqual((w(y+eps)-w(y-eps))/(2*eps),slope,delta=.01)
            for i in range(101):self.assertTrue(0<=w(-h+2*h*i/100)<=1)
            self.assertAlmostEqual((w(eps)-w(0))/eps,(w(0)-w(-eps))/eps,delta=.01)

    def test_radial_rings_are_even_and_boundaries_fixed(self):
        for angle in [i*math.pi/100 for i in range(-100,101)]:
            values=[eye_source_ring_fraction(value,angle) for value in [0,.6,.7,.9,1]]
            self.assertEqual(values,[0,.25,.5,.75,1])
            self.assertTrue(all(b>a for a,b in zip(values,values[1:])))

if __name__=='__main__':unittest.main()
