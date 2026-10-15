import unittest,math
from ear_topology import triangle_quality
class TriangleQuality(unittest.TestCase):
    def test_equilateral(self):self.assertAlmostEqual(triangle_quality((0,0,0),(1,0,0),(.5,math.sqrt(3)/2,0)),1)
    def test_degenerate(self):self.assertEqual(triangle_quality((0,0,0),(1,0,0),(2,0,0)),0)
    def test_scale_invariant(self):self.assertAlmostEqual(triangle_quality((0,0,0),(1,0,0),(0,1,0)),triangle_quality((0,0,0),(2,0,0),(0,2,0)))
