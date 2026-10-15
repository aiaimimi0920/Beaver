import unittest
from ear_fold import plane_target
class FoldPlane(unittest.TestCase):
 def test_projection(self):self.assertEqual(plane_target((1,2,3),(0,0,1),(0,0,1)),(1,2,1))
 def test_unchanged_on_plane(self):self.assertEqual(plane_target((1,2,1),(0,0,1),(0,0,1)),(1,2,1))
