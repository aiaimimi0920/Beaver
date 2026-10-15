import unittest
from ear_boundary import fair_chain
class BoundaryTests(unittest.TestCase):
    def test_bounded_local_fairing(self):
        p=[(-.01,0,0),(0,.004,0),(.01,0,0),(.02,0,0)]
        q=fair_chain(p,{1:[0,2]},[1],limit=.002)
        self.assertEqual(q[0],p[0]);self.assertEqual(q[2:],p[2:])
        self.assertGreaterEqual(q[1][1],.002-1e-12);self.assertLess(q[1][1],.004)
    def test_straight_chain_remains_straight(self):
        p=[(0,0,0),(.002,0,0),(.02,0,0)]
        self.assertEqual(fair_chain(p,{1:[0,2]},[1]),p)
