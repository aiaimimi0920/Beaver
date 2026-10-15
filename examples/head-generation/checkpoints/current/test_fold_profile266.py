import unittest, math
from test_spatial_lashes196 import S, STYLE

class LongitudinalFoldTests(unittest.TestCase):
    def test_fold_tapers_continuously_toward_both_ends(self):
        lo, hi = STYLE['upper_lash_rails'][0][0], STYLE['upper_lash_rails'][-1][0]
        roll = S['main_band_roll']
        self.assertLess(roll(lo), roll((lo+hi)/2))
        self.assertAlmostEqual(roll(lo), roll(hi), places=12)
        samples = [roll(lo+(hi-lo)*i/100) for i in range(101)]
        self.assertLess(max(abs(a-b) for a,b in zip(samples,samples[1:])), .0001)
    def test_variable_fold_keeps_matching_thin_return(self):
        thickness = .00018
        vs,fs,z = S['closed_ribbon_mesh']([[-1,0,.025],[0,0,.025],[1,0,.025]],
            lambda t,h,d:(t*.02,-d,h*.236),thickness,lambda t:.002+.001*(1-t*t),.42,True)
        for start in [0,6,12]:
            for a,b in [(0,1),(2,5),(3,4)]:
                self.assertAlmostEqual(math.dist(vs[start+a],vs[start+b]),thickness,places=12)
        self.assertGreater(S['signed_volume'](vs,fs),0)
