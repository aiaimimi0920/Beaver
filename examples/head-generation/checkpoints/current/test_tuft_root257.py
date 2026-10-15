import unittest
from test_spatial_lashes196 import S, STYLE

class SweptRootReferenceTests(unittest.TestCase):
 def test_declared_scoop_is_bounded_and_tip_stays_forward(self):
  key='upper_swept_tuft_outline'
  outline=STYLE[key]
  tip=max(range(len(outline)),key=lambda i:outline[i][0])
  for side in [-1,1]:
   forward=[-S['upper_accent_point'](side,t,r,0,key)[1] for t,r in outline[:tip+1]]
   self.assertTrue(all(x>=forward[0]-STYLE['accent_tuft_rear_scoop_m']-.0001 for x in forward))
   self.assertGreater(forward[-1],forward[0])

 def test_outline_does_not_project_beyond_tip(self):
  key='upper_swept_tuft_outline';outline=STYLE[key]
  for side in [-1,1]:
   tip=max(outline,key=lambda p:p[0]);limit=-S['upper_accent_point'](side,*tip,0,key)[1]
   for t,r in outline:
    self.assertLessEqual(-S['upper_accent_point'](side,t,r,0,key)[1],limit+.0001)
