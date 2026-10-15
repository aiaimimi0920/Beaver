import unittest
from main_flank import lower_flank_offset
class MainFlank476(unittest.TestCase):
 def test_crest_preserved(self):
  for t in [-1,-.8,-.4,0,.3,.7,1]:self.assertEqual(lower_flank_offset(t,1,.0012,.0018),(0,0))
 def test_tips_preserved(self):
  for t in [-1.15,-.95,.95,1.1]:self.assertEqual(lower_flank_offset(t,0,.0012,.0018),(0,0))
 def test_center_back_and_down(self):self.assertEqual(lower_flank_offset(0,0,.0012,.0018),(.0018,-.0012))
 def test_bounded_symmetric(self):
  for i in range(101):
   t=i/100;a=lower_flank_offset(t,.3,.0012,.0018);b=lower_flank_offset(-t,.3,.0012,.0018)
   self.assertEqual(a,b);self.assertTrue(0<=a[0]<=.0018);self.assertTrue(-.0012<=a[1]<=0)
