import unittest
from source_polygon_validity import polygon_diagonals

class PolygonValidityTests(unittest.TestCase):
    square=[(0,0,0),(1,0,0),(1,1,0),(0,1,0)]
    def test_convex_both_diagonals(self):self.assertEqual(set(polygon_diagonals(self.square,(0,1,2,3))),{0,1})
    def test_reversed_winding_is_valid(self):self.assertEqual(set(polygon_diagonals(self.square,(3,2,1,0))),{0,1})
    def test_bowtie_rejected(self):self.assertFalse(polygon_diagonals(self.square,(0,2,1,3)))
    def test_concave_only_interior_diagonal(self):
        p=[(0,0,0),(1,0,0),(.2,.2,0),(0,1,0)]
        self.assertEqual(set(polygon_diagonals(p,(0,1,2,3))),{0})
    def test_paired_must_share_diagonal(self):
        a=[(0,0,0),(1,0,0),(.2,.2,0),(0,1,0)]
        b=[(0,0,0),(.2,.2,0),(1,1,0),(0,1,0)]
        self.assertFalse(polygon_diagonals(a,(0,1,2,3),b))
    def test_paired_bowtie_rejected(self):
        p=[self.square[i] for i in (0,2,1,3)]
        self.assertFalse(polygon_diagonals(self.square,(0,1,2,3),p))
    def test_degenerate_edge(self):self.assertFalse(polygon_diagonals(self.square,(0,1,1,3)))
    def test_collinear_boundary(self):self.assertFalse(polygon_diagonals([(0,0,0),(1,0,0),(2,0,0),(3,0,0)],(0,1,2,3)))
    def test_touching_boundary(self):self.assertFalse(polygon_diagonals([(0,0,0),(1,0,0),(.5,0,0),(0,1,0)],(0,1,2,3)))
    def test_nonfinite(self):self.assertFalse(polygon_diagonals([(float('nan'),0,0)]+self.square[1:],(0,1,2,3)))
    def test_triangle(self):self.assertTrue(polygon_diagonals(self.square,(0,1,2)))
    def test_actual_fold_rejected(self):
        p=[(.005608647,-.077281907,1.547270894),(.005777147,-.077518687,1.547411442),(.016401280,-.070121869,1.552334666),(.014996973,-.073665500,1.552376509)]
        self.assertFalse(polygon_diagonals(p,(0,1,2,3)))
    def test_rigid_translation(self):
        p=[(x+10,y-4,z+3) for x,y,z in self.square]
        self.assertEqual(set(polygon_diagonals(p,(0,1,2,3))),{0,1})

if __name__=='__main__':unittest.main()
