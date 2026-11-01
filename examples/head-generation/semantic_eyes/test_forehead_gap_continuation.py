"""State machine only: mocked fitter/measurement never certify geometry."""
import copy
import unittest
from unittest.mock import patch
import forehead_gap_continuation as g

class GapContinuationTests(unittest.TestCase):
    def setUp(self):
        self.names=[f'pair{i}:radial' for i in range(10)]
        self.author={'vertices':[[0.,0.,0.]],'open_delta':[[0.,0.,0.]]}
        self.p={'vertices':[[0.,0.,0.]],'paths':{'blue':[0]},'faces':[],
                'protected':[0],'source_chart':[[0.,0.,0.]],'source_faces':[],
                'topology':{'lineage':{},'vertices':[[0.,0.,0.]]},
                'uniform_station_reference_length_mm':{'blue':1.},
                'joint_stage_caps':{'station':{'blue':.1}},
                'matched_gap_caps_v2':{n:.3 for n in self.names},
                'test_errors':{n:.2 for n in self.names}}
    def measure(self,a,p):
        return {'shared_gap_maximum_relative_deviation':dict(p['test_errors']),
                'passed':False,'failures':['physical:blue']}
    def fit(self,*args,**kw):
        self.assertEqual(kw['solver_mode'],'linearized')
        self.assertEqual(kw['progress_names'],[])
        self.assertEqual(kw['gap_sample_count'],129)
        p=args[2];n=kw['active_gap_pair']
        p['matched_gap_caps_v2'][n]-=kw['gap_reduction']
        p['test_errors'][n]=.09
        return p
    def run_driver(self,fit=None,**kw):
        with patch.object(g,'measure_forehead_shared_acceptance',side_effect=self.measure),patch.object(g,'fit_forehead_shared_constraints',side_effect=fit or self.fit):
            return g.continue_forehead_gaps(self.author,self.p,**kw)
    def test_all_domains_component_only_and_input_isolation(self):
        prior=copy.deepcopy((self.author,self.p))
        out=self.run_driver(max_steps=10)
        self.assertTrue(out['transverse_goals_passed']);self.assertFalse(out['full_model_success'])
        self.assertFalse(out['acceptance']['passed']);self.assertEqual(len(out['stages']),10)
        self.assertEqual((self.author,self.p),prior)
    def test_seam_growth_is_explicit_and_forwarded(self):
        for requested in (False, True):
            seen=[]
            def fit(*args, **kw):
                seen.append(kw['seam_active_set_growth'])
                return self.fit(*args, **kw)
            options={'seam_active_set_growth': True} if requested else {}
            self.run_driver(fit, max_steps=1, **options)
            self.assertEqual(seen, [requested])
        with self.assertRaisesRegex(ValueError, 'seam active-set'):
            self.run_driver(seam_active_set_growth=1)
    def test_blocked_pair_does_not_block_other_pair(self):
        def fit(*args,**kw):
            if kw['active_gap_pair']==self.names[0]:raise ValueError('capacity')
            return self.fit(*args,**kw)
        out=self.run_driver(fit,max_steps=1)
        self.assertEqual(len(out['attempts']),5)
        self.assertEqual(out['stages'][0]['domain'],self.names[1])
    def test_rejection_cannot_mutate_author_or_proposal(self):
        prior=copy.deepcopy((self.author,self.p))
        def bad(*args,**kw):
            args[2]['topology']['vertices'][0][0]=999
            kw['author_context']['vertices'][0][0]=999
            raise ValueError('rejected')
        out=self.run_driver(bad,max_steps=1,fractions=(1.,))
        self.assertEqual(len(out['attempts']),10);self.assertEqual(out['proposal'],self.p)
        self.assertEqual((self.author,self.p),prior)
    def test_other_cap_baseline_reference_and_pin_changes_reject(self):
        def change_cap(p):p['joint_stage_caps']['station']['blue']=.11
        def change_field(p):p['topology']['lineage'][0]={'donors':[0],'weights':[1.]}
        def change_ref(p):p['uniform_station_reference_length_mm']['blue']=2.
        def change_pin(p):p['vertices'][0][0]=1.
        def change_other_gap(p):p['matched_gap_caps_v2'][self.names[-1]]=.4
        for change in (change_cap,change_field,change_ref,change_pin,change_other_gap):
            def fit(*args,**kw):
                p=self.fit(*args,**kw);change(p);return p
            out=self.run_driver(fit,max_steps=1,fractions=(1.,))
            self.assertEqual(out['stages'],[]);self.assertEqual(out['proposal'],self.p)
    def test_dense_gate_and_missing_caps_fail_closed(self):
        self.p['test_errors'][self.names[0]]=.31
        with self.assertRaisesRegex(ValueError,'retained dense'):self.run_driver()
        self.p.pop('matched_gap_caps_v2')
        with self.assertRaisesRegex(ValueError,'Explicit ten'):self.run_driver()
    def test_checkpoint_detached_and_io_failure_propagates(self):
        def callback(state):state['proposal']['vertices'][0][0]=999
        out=self.run_driver(max_steps=1,checkpoint=callback)
        self.assertEqual(out['proposal']['vertices'][0][0],0.)
        def failed(state):raise OSError('atomic write failed')
        with self.assertRaisesRegex(OSError,'atomic'):self.run_driver(checkpoint=failed)
    def test_fraction_schedule_and_budget(self):
        calls=[]
        def fit(*args,**kw):
            calls.append(kw['gap_reduction'])
            if kw['gap_reduction']>.00625:raise ValueError('too large')
            return self.fit(*args,**kw)
        out=self.run_driver(fit,max_steps=1)
        self.assertEqual(calls,[.025,.0125,.00625]);self.assertEqual(len(out['stages']),1)
        self.assertIn('budget',out['stop_reason'])

    def test_persisted_fraction_unknown_or_slow_never_retries_full(self):
        self.p['gap_continuation_schedule']={self.names[0]:{'next_fraction':.25}}
        self.p['test_errors']={n:(.2 if n==self.names[0] else .09) for n in self.names}
        calls=[]
        def fit(*args,**kw):
            calls.append(kw['gap_reduction'])
            p=self.fit(*args,**kw);p['test_errors'][self.names[0]]=.2
            p['metrics']={'solver_iterations':2,'solver_iteration_budget':120}
            return p
        out=self.run_driver(fit,max_steps=2)
        self.assertEqual(calls,[.00625,.00625])
        self.assertEqual(out['proposal']['gap_continuation_schedule'][self.names[0]]['next_fraction'],.25)
        record=g.forehead_gap_fraction_record(.25,(1.,.5,.25,.1,.05),{'solver_work_iterations':20})
        self.assertEqual(record['next_fraction'],.25)

    def test_verified_total_work_promotes_only_one_level(self):
        levels=(1.,.5,.25,.1,.05)
        record=g.forehead_gap_fraction_record(.1,levels,{'solver_work_iterations':8})
        self.assertEqual(record['next_fraction'],.25)
        for metadata in ({'solver_work_iterations':True},{'solver_work_iterations':0},
                         {'solver_iterations':1}, {'solver_work_iterations':8,'restoration_continuation':{'used':True}}):
            self.assertEqual(g.forehead_gap_fraction_record(.1,levels,metadata)['next_fraction'],.1)

    def test_invalid_schedule_is_only_hint_and_blocked_domain_still_skipped(self):
        self.p['gap_continuation_schedule']={self.names[0]:{'next_fraction':.1},self.names[1]:{'next_fraction':float('nan')}}
        calls=[]
        def fit(*args,**kw):
            calls.append((kw['active_gap_pair'],kw['gap_reduction']))
            if kw['active_gap_pair']==self.names[0]:raise ValueError('blocked')
            return self.fit(*args,**kw)
        out=self.run_driver(fit,max_steps=1)
        self.assertAlmostEqual(calls[0][1],.0025)
        self.assertAlmostEqual(calls[1][1],.00125)
        self.assertEqual(calls[2],(self.names[1],.025))
        self.assertEqual(len(out['stages']),1)

if __name__=='__main__':unittest.main()
