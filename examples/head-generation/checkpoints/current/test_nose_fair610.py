import unittest
from nose_fair import fit_arc,evaluate_arc
class NoseFair610(unittest.TestCase):
 def test_pinned_endpoints_and_tangents(self):
  points=[(0,0),(.2,.18),(.4,.5),(.6,.8),(.8,.95),(1,1)]
  a=fit_arc(points,0,.2);self.assertEqual(evaluate_arc(0,a),0);self.assertEqual(evaluate_arc(1,a),1)
  e=1e-6;self.assertAlmostEqual((evaluate_arc(e,a)-evaluate_arc(0,a))/e,0,places=4)
  self.assertAlmostEqual((evaluate_arc(1,a)-evaluate_arc(1-e,a))/e,.2,places=4)
 def test_quintic_reproduces_smooth_arc(self):
  points=[(i/10,(i/10)**2) for i in range(11)];a=fit_arc(points,0,2)
  for i in range(101):self.assertAlmostEqual(evaluate_arc(i/100,a),(i/100)**2,places=10)
