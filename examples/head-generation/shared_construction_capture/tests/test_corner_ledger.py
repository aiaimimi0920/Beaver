import copy
import unittest
from corner_ledger import CornerLedger, corner_id, f32


def samples(points, shift=0.):
    return [{'point':p,'values':{'UVMap':[shift+(.5 if p=='b' else 0.),0.]},
             'categorical':{'pin_uv':False}} for p in points]


class CornerTests(unittest.TestCase):
    def setUp(self):
        self.ledger=CornerLedger()
        self.storage={'UVMap':'float32'}
        self.ledger.capture('producer:face:A',['a','b','c'],samples(['a','b','c']),self.storage)
        self.ledger.capture('producer:face:B',['b','a','d'],samples(['b','a','d']),self.storage)
        self.plan=self.ledger.donors(['producer:face:A','producer:face:B'],['a','b','c','d'])

    def test_rotation_same_identity(self):
        before=copy.deepcopy(self.ledger.faces)
        ids=self.ledger.capture('producer:face:A',['b','c','a'],samples(['b','c','a']),self.storage)
        self.assertEqual(before,self.ledger.faces)
        self.assertEqual(ids[0],corner_id('producer:face:A','b','c'))

    def test_winding_change_refused(self):
        with self.assertRaises(ValueError):
            self.ledger.capture('producer:face:A',['a','c','b'],samples(['a','c','b']),self.storage)

    def test_seam_sides_have_distinct_identity(self):
        self.assertNotEqual(corner_id('producer:face:A','a','b'),corner_id('producer:face:B','a','d'))
        self.assertEqual(len(self.plan['alternatives']['a']),2)
        self.assertEqual(self.plan['chosen']['a'],corner_id('producer:face:A','a','b'))
        flipped=self.ledger.donors(['producer:face:B','producer:face:A'],['a','b','c','d'])
        self.assertEqual(flipped['chosen']['a'],corner_id('producer:face:B','a','d'))

    def test_different_uv_seam_not_collapsed(self):
        self.ledger.capture('other:face',['b','a','e'],samples(['b','a','e'],.2),self.storage)
        with self.assertRaisesRegex(ValueError,'seam'):
            self.ledger.donors(['producer:face:A','other:face'],['a','b','c','e'])

    def test_categorical_boundary_refusal(self):
        ss=samples(['b','a','e']);ss[0]['categorical']['pin_uv']=True
        self.ledger.capture('other:face',['b','a','e'],ss,self.storage)
        with self.assertRaisesRegex(ValueError,'Categorical'):
            self.ledger.donors(['producer:face:A','other:face'],['a','b','c','e'])

    def birth(self,plan=None,wrong=False):
        ss=samples(['new-a','new-b','new-c'])
        ss[0]['values']['UVMap']=[.25 if not wrong else .3,0.]
        ss[1]['values']['UVMap']=[.5,0.]
        return self.ledger.allocate('allocation:event:face:1',['new-a','new-b','new-c'],ss,
                                    [{'a':.5,'b':.5},{'b':1.},{'c':1.}],plan or self.plan)

    def test_birth_explicit_corner_stencils(self):
        ids=self.birth()
        expected={self.plan['chosen']['a']:.5,self.plan['chosen']['b']:.5}
        self.assertEqual(self.ledger.births[ids[0]]['source_corners'],expected)
        self.assertEqual(self.ledger.births[ids[0]]['expected_float32_values']['UVMap'],[.25,0.])

    def test_failed_birth_does_not_commit(self):
        before=copy.deepcopy(self.ledger.faces)
        with self.assertRaises(ValueError):self.birth(wrong=True)
        self.assertEqual(self.ledger.faces,before)
        self.assertFalse(self.ledger.births)

    def test_mutable_plan_fabricated_lineage_refused(self):
        plan=copy.deepcopy(self.plan)
        # a and c have identical UV values, so numerical comparison cannot catch this.
        plan['chosen']['a']=plan['chosen']['c']
        with self.assertRaisesRegex(ValueError,'Donor plan'):
            self.birth(plan=plan)

    def test_byte_color_unknown_storage_refused(self):
        with self.assertRaisesRegex(ValueError,'BYTE_COLOR'):
            self.ledger.capture('new',['a','b','e'],samples(['a','b','e']),{'UVMap':'byte_color'})

    def test_float32_storage_verified(self):
        self.assertNotEqual(f32(.1),.1)
        ss=samples(['x','y','z']);ss[0]['values']['UVMap']=[f32(.1),0.]
        self.ledger.allocate('new',['x','y','z'],ss,
                             [{'a':.8,'b':.2},{'a':1.},{'c':1.}],self.plan)


if __name__=='__main__':
    unittest.main()
