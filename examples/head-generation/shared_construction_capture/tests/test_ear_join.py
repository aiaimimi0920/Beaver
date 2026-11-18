import unittest
from copy import deepcopy
from ear_join import expected_membership

class MembershipTests(unittest.TestCase):
    def setUp(self):
        self.before={'groups':[[0,'Other'],[1,'Continuous facial shell']],
                     'vertices':[[[0,0,0],False,False,[[0,.25]]],[[1,0,0],False,False,[[1,.4]]]],
                     'untouched':['UV','geometry','fields']}
        self.skin={'groups':[[2,'Continuous facial shell']],
                   'vertices':[[[0,0,0],False,False,[[2,1.]]]]}
        self.targets={'rows':[{'component':'ear:1','skin_point_id':'skin','target_index':0}]}
    def test_exact_named_group_only(self):
        old=deepcopy(self.before)
        result=expected_membership(self.before,self.skin,['skin'],self.targets)
        self.assertEqual(result['vertices'][0][3],[[0,.25],[1,1.]])
        self.assertEqual(result['vertices'][1],old['vertices'][1]);self.assertEqual(self.before,old)
        self.assertEqual(result['untouched'],old['untouched'])
    def test_nonunit_authority_rejected(self):
        self.skin['vertices'][0][3][0][1]=.5
        with self.assertRaises(ValueError):expected_membership(self.before,self.skin,['skin'],self.targets)
    def test_absent_group_rejected(self):
        self.before['groups']=[]
        with self.assertRaises(ValueError):expected_membership(self.before,self.skin,['skin'],self.targets)
    def test_oral_group_not_changed(self):
        self.targets['rows'][0]['component']='oral'
        self.assertEqual(expected_membership(self.before,self.skin,['skin'],self.targets),self.before)

if __name__=='__main__':unittest.main()
