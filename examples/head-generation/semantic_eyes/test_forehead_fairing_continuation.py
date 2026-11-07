import copy
import unittest
from forehead_fairing_continuation import continue_forehead_fairing


class FairingContinuationTests(unittest.TestCase):
    def test_forwarding_and_persistent_reference(self):
        source={'vertices':[[0,0,0]],'metrics':{}}
        calls=[]
        def fit(v,d,p,**kw):
            calls.append(copy.deepcopy((p,kw)))
            p.setdefault('whole_guide_fairing_reference',{'persistent':True})
            p['metrics']['guide_fairing']={'initial_energy':2-len(calls)*.01,'final_energy':1.9}
            return p
        result=continue_forehead_fairing([],[],source,{},max_stages=2,fitter=fit)
        self.assertEqual(len(result['stages']),2)
        self.assertNotIn('whole_guide_fairing_reference',calls[0][0])
        self.assertEqual(calls[1][0]['whole_guide_fairing_reference'],{'persistent':True})
        self.assertEqual(calls[0][1]['progress_names'],[])
        self.assertEqual(calls[0][1]['gap_reduction'],0)
        self.assertEqual(calls[0][1]['minimum_improvement'],0)
        self.assertEqual(source,{'vertices':[[0,0,0]],'metrics':{}})
        self.assertTrue(result['fit_complete']);self.assertTrue(result['component_admitted']);self.assertFalse(result['full_model_success'])

    def test_rejection_isolation(self):
        def fit(v,d,p,**kw):
            p['vertices'][0][0]=99
            raise ValueError('Rejected current iterate')
        source={'vertices':[[0,0,0]],'metrics':{}}
        result=continue_forehead_fairing([],[],source,{},fitter=fit)
        self.assertEqual(result['proposal'],source)
        self.assertEqual(result['stages'],[])
        self.assertIn('Rejected',result['attempts'][0]['reason'])

    def two_roles(self):
        return {'vertices': [[0,0,0],[1,0,0],[1,1,0],[0,0,1],[1,0,1],[1,1,1]],
                'paths': {'orange':[0,1,2],'purple':[3,4,5]}, 'metrics': {}}

    def test_blocked_role_does_not_starve_other_role(self):
        calls=[]
        def fit(v,d,p,**kw):
            calls.append((kw['fairing_turn_roles'],kw['fairing_turn_reduction']))
            if kw['fairing_turn_roles']==['orange']:
                p['vertices'][0][0]=99
                raise ValueError('Blocked orange')
            self.assertEqual(p['vertices'][0][0],0)
            p['metrics']={'guide_fairing':{},'solver_work_iterations':30}
            return p
        result=continue_forehead_fairing([],[],self.two_roles(),{},turn_goals={'orange':10.,'purple':20.},turn_reduction_degrees=1.,fitter=fit)
        self.assertEqual(calls,[(['orange'],1.),(['orange'],.5),(['orange'],.25),(['purple'],1.)])
        self.assertTrue(result['component_admitted'])
        self.assertEqual(result['blocked_roles'],[])

    def test_slow_success_reuses_fraction_and_quick_success_grows_one_level(self):
        for work, expected in ((30,.25),(10,.5)):
            source=self.two_roles();source['whole_guide_fairing_schedule']={'orange':.25}
            calls=[]
            def fit(v,d,p,**kw):
                calls.append(kw['fairing_turn_reduction'])
                p['metrics']={'guide_fairing':{},'solver_work_iterations':work}
                return p
            result=continue_forehead_fairing([],[],source,{},max_stages=2,
                turn_goals={'orange':10.,'purple':20.},turn_reduction_degrees=1.,fitter=fit)
            self.assertEqual(calls,[.25,expected])

    def test_all_blocked_is_finite_and_diagnostics_tagged(self):
        records=[]
        def fit(v,d,p,**kw):
            kw['diagnostic']({'stage':'solver_end'})
            raise ValueError('Blocked')
        result=continue_forehead_fairing([],[],self.two_roles(),{},turn_goals={'orange':10.,'purple':20.},turn_reduction_degrees=1.,fitter=fit,diagnostic=records.append)
        self.assertEqual(len(result['attempts']),6)
        self.assertEqual(result['blocked_roles'],['orange','purple'])
        self.assertEqual([r['fairing_attempt']['sequence'] for r in records],list(range(6)))
        self.assertFalse(result['component_admitted'])

    def test_bounded(self):
        for n in (0,21,True):
            with self.assertRaises(ValueError):continue_forehead_fairing([],[],{}, {},max_stages=n)

if __name__=='__main__':unittest.main()
