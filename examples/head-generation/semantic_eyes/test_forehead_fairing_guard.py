import copy,json,unittest
import numpy as np
from forehead_fairing_stage import ForeheadFairingStage
from forehead_fairing_guard import forehead_capture_fairing_guard, forehead_prepare_fairing_stage, forehead_migrate_support_fairing_guard, forehead_fairing_guard_retains, forehead_migrate_fairing_guard_v2


class FairingGuardTests(unittest.TestCase):
    def setUp(self):
        p=np.array([[0.,0.,0.],[.01,.002,0.],[.02,-.001,0.],[.03,0.,0.],[.01,.01,0.]])
        self.p={'vertices':p.tolist(),'faces':[[0,1,4,3]],'paths':{'rail':[0,1,2,3]},'uniform_station_reference_length_mm':{'rail':32.},'topology':{'lineage':{'4':{'donors':[0,1,2,3],'weights':[.25]*4}}}}
        stage=ForeheadFairingStage(p,self.p['paths'],self.p['uniform_station_reference_length_mm'])
        stage.target_energy=stage.initial_energy
        self.p['whole_guide_fairing_reference']=stage.reference
        self.p['metrics']={'guide_fairing':stage.evidence(p)}
        self.p['whole_guide_fairing_guard']=forehead_capture_fairing_guard(self.p,stage)

    def test_numpy_migration_hash_matches_serialized_guard(self):
        p=copy.deepcopy(self.p)
        expected=forehead_migrate_fairing_guard_v2(p)
        p['whole_guide_fairing_guard']['turn_caps_degrees']=np.asarray(p['whole_guide_fairing_guard']['turn_caps_degrees'])
        p['vertices']=np.asarray(p['vertices'])
        actual=forehead_migrate_fairing_guard_v2(p)
        self.assertEqual(actual['migration'],expected['migration'])

    def test_explicit_v2_mode_and_unknown_versions(self):
        with self.assertRaises(ValueError):forehead_prepare_fairing_stage(self.p,self.p['vertices'],guard_mode='role-max-bv-v2')
        p=copy.deepcopy(self.p);p['whole_guide_fairing_guard']=forehead_migrate_fairing_guard_v2(p)
        stage=forehead_prepare_fairing_stage(p,p['vertices'],guard_mode='role-max-bv-v2')
        self.assertEqual(stage.reference,self.p['whole_guide_fairing_reference'])
        self.assertTrue(stage.accepts(p['vertices']))
        for version in (0,3,True,'2'):
            invalid=copy.deepcopy(p);invalid['whole_guide_fairing_guard']['version']=version
            with self.assertRaises(ValueError):forehead_prepare_fairing_stage(invalid,invalid['vertices'])

    def test_migration_rejects_violated_v1_and_v2_bv_caps_retained(self):
        bad=copy.deepcopy(self.p);bad['vertices'][1][1]*=2
        with self.assertRaises(ValueError):forehead_migrate_fairing_guard_v2(bad)
        p=copy.deepcopy(self.p);p['whole_guide_fairing_guard']=forehead_migrate_fairing_guard_v2(p)
        q=copy.deepcopy(p);q['whole_guide_fairing_guard']['bending_variation_caps'][0]+=.1
        with self.assertRaises(ValueError):forehead_fairing_guard_retains(p,q)
        q=copy.deepcopy(p);q['vertices'].append([0,.02,0]);q['faces']=[[0,1,4,5],[0,5,4,3]]
        q['whole_guide_fairing_guard']=forehead_migrate_support_fairing_guard(p,q)
        self.assertEqual(q['whole_guide_fairing_guard']['migration'],p['whole_guide_fairing_guard']['migration'])
        self.assertEqual(q['whole_guide_fairing_guard']['bending_variation_caps'],p['whole_guide_fairing_guard']['bending_variation_caps'])

    def test_cross_phase_rejection(self):
        forehead_fairing_guard_retains(self.p,copy.deepcopy(self.p))
        for key in ('energy_cap','turn_caps_degrees'):
            after=copy.deepcopy(self.p)
            if key=='energy_cap':after['whole_guide_fairing_guard'][key]+=.001
            else:after['whole_guide_fairing_guard'][key][0]+=.001
            with self.assertRaises(ValueError):forehead_fairing_guard_retains(self.p,after)
        after=copy.deepcopy(self.p);del after['whole_guide_fairing_guard']
        with self.assertRaises(ValueError):forehead_fairing_guard_retains(self.p,after)

    def test_ordinary_phase_retains_exact_caps_after_serialization(self):
        p=json.loads(json.dumps(self.p));stage=forehead_prepare_fairing_stage(p,p['vertices'])
        self.assertEqual(stage.target_energy,p['whole_guide_fairing_guard']['energy_cap'])
        np.testing.assert_array_equal(stage.turn_caps,p['whole_guide_fairing_guard']['turn_caps_degrees'])
        self.assertTrue(stage.accepts(p['vertices']))
        changed=np.asarray(p['vertices']);changed[1,1]*=2
        self.assertFalse(stage.accepts(changed))

    def test_no_silent_legacy_reset(self):
        p=copy.deepcopy(self.p);del p['whole_guide_fairing_guard']
        with self.assertRaises(ValueError):forehead_prepare_fairing_stage(p,p['vertices'],.005)
        p['whole_guide_fairing_guard']=forehead_capture_fairing_guard(p)
        self.assertTrue(forehead_prepare_fairing_stage(p,p['vertices']).accepts(p['vertices']))

    def test_explicit_support_migration(self):
        p=copy.deepcopy(self.p);p['vertices'].append([.01,.02,0.]);p['faces']=[[0,1,4,5],[0,5,4,3]]
        with self.assertRaises(ValueError):forehead_prepare_fairing_stage(p,p['vertices'])
        p['whole_guide_fairing_guard']=forehead_migrate_support_fairing_guard(self.p,p)
        stage=forehead_prepare_fairing_stage(p,p['vertices'])
        self.assertEqual(stage.reference,self.p['whole_guide_fairing_reference'])
        self.assertEqual(stage.target_energy,self.p['whole_guide_fairing_guard']['energy_cap'])

    def test_guide_movement_topology_and_baseline_fail_closed(self):
        for mode in ('move','path','field'):
            p=copy.deepcopy(self.p)
            if mode=='move':p['vertices'][1][1]+=.0001
            elif mode=='path':p['paths']['rail']=[0,2,1,3]
            else:p['topology']['lineage']['4']['weights']=[.4,.2,.2,.2]
            with self.assertRaises(ValueError):forehead_migrate_support_fairing_guard(self.p,p)

    def test_repeated_stage_keeps_normalizers_and_tightens(self):
        stage=forehead_prepare_fairing_stage(self.p,self.p['vertices'],.005)
        self.assertEqual(stage.reference,self.p['whole_guide_fairing_reference'])
        self.assertAlmostEqual(stage.target_energy,.995*self.p['whole_guide_fairing_guard']['energy_cap'])

if __name__=='__main__':unittest.main()
