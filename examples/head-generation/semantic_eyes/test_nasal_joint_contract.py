"""Final-domain rejection tests independent of saved character point IDs."""
import unittest
import numpy as np
from nasal_joint_contract import nasal_oriented_cycle, validate_nasal_joint_domain


class NasalJointContractTests(unittest.TestCase):
    def setUp(self):
        self.v = np.array([(x,0,z) for z in (0,.02) for x in (0,.02,.04)])
        self.faces = [[0,1,4,3],[1,2,5,4]]
        self.regular = dict(affected_faces=[0,1],outer_boundary_fixed=True,
                            protected_vertices_unchanged=True,maximum_move_m=0.)
        self.inner = dict(removed_faces=[1],boundary_positive=[1,2,5,4],
                          new_quads=[self.faces[1]],protected_first_support_untouched=True)

    def check(self, final=None, points=None):
        return validate_nasal_joint_domain(self.v,self.faces,[1,1],self.regular,self.inner,
                                          self.v.copy() if points is None else points,
                                          self.faces if final is None else final)

    def test_valid_final_domain_and_oriented_cycles(self):
        proof = self.check()
        self.assertEqual(proof['quality']['invalid_faces'], [])
        self.assertEqual(proof['removed_cycles'], [nasal_oriented_cycle(self.faces[1])])
        self.assertEqual(nasal_oriented_cycle([1,4,3,0]),nasal_oriented_cycle(self.faces[0]))
        self.assertNotEqual(nasal_oriented_cycle(self.faces[0][::-1]),nasal_oriented_cycle(self.faces[0]))

    def test_missing_surviving_neighbor_rejected(self):
        with self.assertRaisesRegex(ValueError,'ownership mismatch'):
            self.check(final=[self.faces[1]])

    def test_retained_coordinate_change_rejected(self):
        p = self.v.copy();p[0,1] = .0001
        with self.assertRaisesRegex(ValueError,'retained'):
            self.check(points=p)

    def test_invalid_new_quad_cannot_be_deferred(self):
        self.inner['new_quads'] = [[1,5,2,4]]
        with self.assertRaisesRegex(ValueError,'geometry failed'):
            self.check(final=[self.faces[0],self.inner['new_quads'][0]])

    def test_protected_point_violation_rejected(self):
        self.regular['protected_vertices_unchanged'] = False
        with self.assertRaisesRegex(ValueError,'bounds changed'):
            self.check()


if __name__ == '__main__':
    unittest.main()
