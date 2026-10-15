"""The new free-edge opening must never displace the attached eyelid root."""
import unittest, math
from test_spatial_lashes196 import S,STYLE
class FreeEdgeTests(unittest.TestCase):
 def test_envelope_root_is_invariant_and_opening_is_bounded(self):
  blend=STYLE['accent_free_edge_envelope_blend']
  try:
   for t in [-.85,-.5,0,.5,.85]:
    lo=S['curve'](t,1,STYLE['upper_lash_rails']);hi=S['curve'](t,2,STYLE['upper_lash_rails'])
    for side in [-1,1]:
     STYLE['accent_free_edge_envelope_blend']=0
     oldroot=S['upper_accent_point'](side,t,hi,0);oldtip=S['upper_accent_point'](side,t,lo,0)
     STYLE['accent_free_edge_envelope_blend']=blend
     root=S['upper_accent_point'](side,t,hi,0);tip=S['upper_accent_point'](side,t,lo,0)
     self.assertLess(math.dist(root,oldroot),1e-12)
     self.assertAlmostEqual(tip[0],oldtip[0]);self.assertAlmostEqual(tip[2],oldtip[2])
     self.assertGreaterEqual(oldtip[1]-tip[1],-1e-12)
     self.assertLessEqual(oldtip[1]-tip[1],STYLE['accent_free_edge_envelope_m']*blend+1e-12)
  finally:STYLE['accent_free_edge_envelope_blend']=blend

 def test_main_front_surface_has_upward_geometric_normal(self):
  for t in [-.5,0,.5]:
   lo=S['curve'](t,1,STYLE['upper_lash_rails']); hi=S['curve'](t,2,STYLE['upper_lash_rails'])
   a=S['upper_accent_point'](1,t,lo,0);b=S['upper_accent_point'](1,t+.001,lo,0);c=S['upper_accent_point'](1,t,hi,0)
   u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)]
   ny=u[2]*v[0]-u[0]*v[2];nz=u[0]*v[1]-u[1]*v[0]
   if ny>0:ny=-ny;nz=-nz
   self.assertLess(ny,0);self.assertGreater(nz,0)
