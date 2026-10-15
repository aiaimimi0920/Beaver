import unittest,math
from test_spatial_lashes196 import S,STYLE
class InnerForkSupportTests(unittest.TestCase):
    def test_root_unchanged_and_tip_follows_surface(self):
        old=STYLE['accent_inner_fork_surface_follow']
        try:
            for side in [-1,1]:
                outline=STYLE['inner_lash_fork_outline']
                root=max(outline,key=lambda p:p[0]);tip=min(outline,key=lambda p:p[0])
                STYLE['accent_inner_fork_surface_follow']=0
                before=[S['upper_accent_point'](side,t,h,0,'inner_lash_fork_outline') for t,h in [root,tip]]
                STYLE['accent_inner_fork_surface_follow']=old
                after=[S['upper_accent_point'](side,t,h,0,'inner_lash_fork_outline') for t,h in [root,tip]]
                self.assertLess(math.dist(before[0],after[0]),1e-12)
                self.assertAlmostEqual(before[1][0],after[1][0])
                self.assertAlmostEqual(before[1][2],after[1][2])
                self.assertGreater(after[1][1],before[1][1])
        finally:STYLE['accent_inner_fork_surface_follow']=old
