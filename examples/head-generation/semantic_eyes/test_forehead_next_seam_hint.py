import copy
import unittest
import numpy as np
from forehead_seam_hint import forehead_next_stage_seam_hint, forehead_revalidate_seam_hint

class Tests(unittest.TestCase):
    def setUp(self):
        self.selection={'source_identity':'source','held_vertices':[8],'held_columns':[0,1],
            'evidence':[{'vertex':8,'accepted_parameters_mm':[1.,2.],
                         'frozen_field_reference_m':[0.,0.,0.]}]}
        self.points=np.zeros((10,3));self.params=np.array([[np.nextafter(1.,2.),2.]])
    def make(self,**kwargs):
        args=dict(selection=self.selection,parameters_mm=self.params,variable_ids=[8],
            frozen_field=np.zeros((1,3)),current_points=self.points,prior_points=self.points,
            source_identity='source');args.update(kwargs);return forehead_next_stage_seam_hint(**args)
    def test_new_representation_exact_and_old_proof_unchanged(self):
        old=copy.deepcopy(self.selection);hint=self.make();self.assertEqual(old,self.selection)
        self.assertEqual(hint['evidence'][0]['accepted_parameters_mm'],self.params[0].tolist())
        def sample(x):return np.array([[.000498999,0.,0.]]),np.zeros((1,3,2)),np.zeros(1,bool)
        args=(self.params,[8],np.zeros((1,3)),[8],{'rail':[0,1]},[0],sample)
        rejected=forehead_revalidate_seam_hint(old,*args,source_identity='source',previous_source_identity='source')
        accepted=forehead_revalidate_seam_hint(hint,*args,source_identity='source',previous_source_identity='source')
        self.assertEqual(rejected['held_columns'],[]);self.assertEqual(accepted['held_columns'],[0,1])
    def test_world_change_rejected(self):
        p=self.points.copy();p[8,0]=1e-8
        with self.assertRaisesRegex(ValueError,'world'):self.make(current_points=p)
    def test_baseline_change_rejected(self):
        with self.assertRaisesRegex(ValueError,'baseline'):self.make(frozen_field=np.ones((1,3)))
    def test_source_identity_change_rejected(self):
        with self.assertRaisesRegex(ValueError,'source'):self.make(source_identity='changed')

if __name__=='__main__':unittest.main()
