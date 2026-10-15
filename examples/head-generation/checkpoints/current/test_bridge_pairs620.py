import unittest
from bridge_pairs import paired_bridge_indices
class BridgePairs620(unittest.TestCase):
 def setUp(self):
  self.points=[(-1,0,0),(-1,.1,0),(-1,0,.1),(1,0,0),(1,.1,0),(1,0,.1)]
  self.candidates={10:[0,1,2],20:[3,4,5]}
 def test_one_bad_side_still_preserves_symmetric_pair(self):
  self.assertEqual(set(paired_bridge_indices(self.points,self.candidates,[10])),{10,20})
 def test_two_bad_sides_unchanged(self):
  self.assertEqual(paired_bridge_indices(self.points,self.candidates,[10,20]),[10,20])
 def test_missing_mirror_is_rejected(self):
  with self.assertRaises(AssertionError):paired_bridge_indices(self.points,{10:[0,1,2]},[10])
 def test_asymmetry_is_rejected(self):
  self.points[3]=(1.01,0,0)
  with self.assertRaises(AssertionError):paired_bridge_indices(self.points,self.candidates,[10,20])
