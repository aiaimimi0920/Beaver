import unittest
from types import SimpleNamespace
from test_spatial_lashes196 import S,STYLE,C
from test_style_math import pure_function
class EvaluatedSupportTests(unittest.TestCase):
    def test_distant_back_wall_is_not_a_lid_support(self):
        class Tree:
            def ray_cast(self,*args):return SimpleNamespace(y=.3),None,None,None
        fn,scope=pure_function('eye_accents.py','upper_attached_depth',dict(S,Vector=lambda x:x,_EVALUATED_LID_SUPPORT=Tree()))
        x=.04;y=1.54;th=.00018
        self.assertAlmostEqual(fn(x,y,th),S['upper_lid_support'](x,y)+th+STYLE['accent_surface_clearance_m'])
    def test_outer_leaf_samples_local_corner_not_rear_shell(self):
        rays=[]
        class Tree:
            def ray_cast(self,start,*args):rays.append(start);return None,None,None,None
        fn,scope=pure_function('eye_accents.py','upper_attached_depth',dict(S,Vector=lambda x:x,_EVALUATED_LID_SUPPORT=Tree()))
        fn(.2,1.54,.00018)
        limit=C['height_m']*(C['eye_center_x']+C['eye_half_width']*STYLE['upper_lash_rails'][-1][0])
        self.assertAlmostEqual(rays[0][0],limit)

    def test_authored_root_uses_actual_station(self):
        rays=[]
        class Tree:
            def ray_cast(self,start,*args):rays.append(start);return None,None,None,None
        fn,scope=pure_function('eye_accents.py','upper_attached_depth',dict(S,Vector=lambda x:x,_EVALUATED_LID_SUPPORT=Tree()))
        x=C['height_m']*(C['eye_center_x']+C['eye_half_width']*1.1)
        fn(x,1.54,.00018)
        self.assertAlmostEqual(rays[0][0],x)

    def test_support_probe_is_continuous_at_authored_endpoint(self):
        rays=[]
        class Tree:
            def ray_cast(self,start,*args):rays.append(start);return None,None,None,None
        fn,scope=pure_function('eye_accents.py','upper_attached_depth',dict(S,Vector=lambda x:x,_EVALUATED_LID_SUPPORT=Tree()))
        for t in [1.2-1e-7,1.2,1.2+1e-7]:
            fn(C['height_m']*(C['eye_center_x']+C['eye_half_width']*t),1.54,.00018)
        self.assertLess(max(r[0] for r in rays)-min(r[0] for r in rays),1e-8)
