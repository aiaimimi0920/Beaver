import unittest
from ear_fair import solve,quadratic_intercept
class SectionFit(unittest.TestCase):
 def test_solver(self):self.assertEqual(solve([[1,0],[0,2]],[3,8]),[3,4])
 def test_singular(self):self.assertIsNone(solve([[1,1],[2,2]],[3,6]))
 def test_quadratic_preserved(self):
  samples=[(u,v,.07+.01*u+.02*v+.03*u*u-.01*u*v+.005*v*v,1) for u in [-1,-.5,0,.5,1] for v in [-1,-.5,0,.5,1]]
  self.assertAlmostEqual(quadratic_intercept(samples),.07,places=8)
