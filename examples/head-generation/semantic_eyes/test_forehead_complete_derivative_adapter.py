import unittest
from unittest.mock import patch
from types import SimpleNamespace
import numpy as np
from scipy.sparse import csr_matrix
from forehead_complete_derivative_adapter import (forehead_complete_counted_batch,
    forehead_complete_differential_state, forehead_complete_analytic_objective)
from forehead_complete_split_jacobian import (forehead_complete_fallback_block,
    forehead_complete_base_derivative)
from test_forehead_complete_guide_stage import fixture
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage


class Counter:
    def __init__(self, limit=1000):
        self.calls = []
        self.limit = limit
    def charge(self, kind):
        if len(self.calls) >= self.limit:
            raise ValueError('budget exhausted')
        self.calls.append(kind)
    def constraints(self, x):
        self.charge('constraints')
        return np.array([2*x[0]+3*x[1]+5*x[2], 7*x[0]+11*x[1]+13*x[2]])


class CompleteDerivativeAdapterTests(unittest.TestCase):
    def test_scaled_initial_cache_hits_are_counted_and_bounded(self):
        from forehead_hybrid_scaling import forehead_prepare_hybrid_scaling
        from forehead_complete_guide_fit import forehead_complete_counted_derivative
        stage=Counter(2)
        derivative,metadata=forehead_prepare_hybrid_scaling(lambda x:csr_matrix([[2.]]),np.array([1.]),np.ones(1))
        self.assertIs(forehead_complete_counted_derivative(None,derivative),derivative)
        counted=forehead_complete_counted_derivative(stage,derivative)
        counted(np.array([1.]));counted(np.array([1.]))
        self.assertEqual(metadata['initial_cache_hits'],2)
        with self.assertRaisesRegex(ValueError,'budget'):counted(np.array([1.]))
        self.assertEqual(metadata['initial_cache_hits'],2)

    def test_split_fallback_no_cross_talk_or_analytic_overwrite(self):
        stage = Counter()
        analytic = np.array([[0.,3,5],[7,0,13]])
        fallback = np.array([[True,False,False],[False,True,False]])
        result = forehead_complete_fallback_block(stage, np.array([1.,2,3]), lambda x:(x,),
                                                  analytic, fallback, np.ones((2,3),bool))
        np.testing.assert_allclose(result, [[2,3,5],[7,11,13]],rtol=1e-6)
        self.assertEqual(stage.calls.count('complete_constraint_fallback_probe'),3)
        self.assertEqual(stage.calls.count('constraints'),3)

    def test_empty_and_all_fallback(self):
        stage = Counter()
        expected = np.array([[2.,3,5],[7,11,13]])
        none = np.zeros((2,3),bool)
        np.testing.assert_array_equal(forehead_complete_fallback_block(stage,np.ones(3),None,expected,none,~none),expected)
        self.assertEqual(stage.calls,[])
        result=forehead_complete_fallback_block(stage,np.ones(3),lambda x:(x,),np.zeros((2,3)),~none,~none)
        np.testing.assert_allclose(result,expected,rtol=1e-6)
        self.assertEqual(stage.calls.count('complete_constraint_fallback_probe'),4)

    def test_invalid_and_budget_exhausted_probes_reject(self):
        for stage,evaluate,error in [(Counter(),lambda x:None,'outside source'),(Counter(0),lambda x:(x,),'budget')]:
            with self.assertRaisesRegex(ValueError,error):
                forehead_complete_fallback_block(stage,np.ones(3),evaluate,np.zeros((2,3)),np.ones((2,3),bool),np.ones((2,3),bool))

    def test_counted_source_proxy_preserves_absent_default(self):
        raw=SimpleNamespace(sample=lambda x:x, sample_differential=lambda x:None)
        self.assertIs(forehead_complete_counted_batch(None,raw,'geometry'),raw)
        stage=Counter();wrapped=forehead_complete_counted_batch(stage,raw,'geometry')
        np.testing.assert_array_equal(wrapped.sample(np.ones((1,3))),np.ones((1,3)))
        with self.assertRaises(ValueError):wrapped.sample_differential(np.zeros((1,2)))
        self.assertEqual(stage.calls,['geometry_source_primal','geometry_source_differential'])

    def test_source_differential_mismatch_rejects_and_counts(self):
        stage=Counter();raw=SimpleNamespace(sample_differential=lambda x:(np.ones((1,3)),np.ones((1,3,2)),np.zeros(1,bool)))
        wrapped=forehead_complete_counted_batch(stage,raw,'geometry')
        with self.assertRaisesRegex(ValueError,'mismatch'):
            forehead_complete_differential_state(stage,np.zeros(2),lambda x:(np.zeros((1,3)),),wrapped,[0])
        self.assertEqual(stage.calls,['complete_differential_state_wrapper','geometry_source_differential'])

    def test_objective_arithmetic_drift_or_nonfinite_rejected(self):
        stage=Counter();points=np.zeros((1,3));batch=SimpleNamespace(sample_differential=lambda x:(points,np.ones((1,3,2)),np.zeros(1,bool)))
        for primal in (float('nan'),float('inf'),2.):
            with patch('forehead_complete_derivative_adapter.forehead_cd_objective',return_value=(1.,points,[],{'analytic_primal':primal})):
                with self.assertRaisesRegex(ValueError,'primal mismatch'):
                    forehead_complete_analytic_objective(stage,np.zeros(2),lambda x:(points,),batch,[0])

    def test_constraint_energy_row_rejects_primal_drift(self):
        from forehead_complete_constraint_differential import forehead_cd_parameter_constraints
        p,proposal,config=fixture();stage=ForeheadCompleteGuideFairingStage(proposal,p,'source',config)
        original=stage.energy_from
        with patch.object(stage,'energy_from',side_effect=lambda measured:original(measured)+1.):
            with self.assertRaisesRegex(ValueError,'primal mismatch'):
                forehead_cd_parameter_constraints(stage,p,[2],np.ones((1,3,2)),np.zeros(1,bool))

    def test_actual_source_units_and_dynamic_objective_fallback(self):
        p,proposal,config=fixture();stage=ForeheadCompleteGuideFairingStage(proposal,p,'source',config)
        ids=[2];J=np.array([[[1.,0],[0,1],[0,0]]]);boundary=[False]
        def evaluate(x):
            stage.charge('source_evaluate_including_cached_and_FD')
            q=p.copy();q[2,:2]+=x/1000
            return(q,)
        raw=SimpleNamespace(sample_differential=lambda x:(p[ids]+np.c_[x,np.zeros(1)],J,np.array(boundary)))
        batch=forehead_complete_counted_batch(stage,raw,'geometry')
        x=np.zeros(2);gradient=forehead_complete_analytic_objective(stage,x,evaluate,batch,ids)
        expected=[]
        for c in range(2):
            a=x.copy();b=x.copy();a[c]+=1e-4;b[c]-=1e-4
            expected.append((stage.objective(evaluate(a)[0])-stage.objective(evaluate(b)[0]))/2e-4)
        np.testing.assert_allclose(gradient,expected,rtol=1e-5,atol=1e-6)
        before=stage.calls.get('complete_objective_fallback_probe',0);boundary[0]=True
        second=forehead_complete_analytic_objective(stage,x,evaluate,batch,ids)
        self.assertEqual(stage.calls['complete_objective_fallback_probe']-before,4)
        np.testing.assert_allclose(second,expected,rtol=1e-6)
        self.assertGreater(stage.calls['objective_differential_production_primal'],0)
        self.assertEqual(stage.total_calls,sum(stage.calls.values()))

    def test_other_rows_are_separate_scaled_and_masks_rebuilt(self):
        stage=Counter();calls=[];x=np.array([1.,2.,3.]);pattern=csr_matrix(np.ones((4,3),bool))
        def other(z):
            calls.append(z.copy());return np.array([17*z[0],0,0,19*z[2]])
        flag=np.array([[True,False,False],[False,True,False]])
        def assembly(*args):
            return None,np.array([[2.,3,5],[7,11,13]]),flag.copy(),np.ones((2,3),bool),{}
        with patch('forehead_complete_split_jacobian.forehead_complete_analytic_constraints',side_effect=assembly) as callback:
            derivative=forehead_complete_base_derivative(stage,other,lambda z:(z,),None,None,(1,3),np.array([2.,3,4,5]),pattern)
            actual=derivative(x).toarray();flag[:]=False;again=derivative(x).toarray()
        expected=np.array([[17/2,0,0],[2/3,1,5/3],[7/4,11/4,13/4],[0,0,19/5]])
        np.testing.assert_allclose(actual,expected,rtol=1e-6,atol=1e-6)
        np.testing.assert_allclose(again,expected,rtol=1e-6,atol=1e-6)
        self.assertEqual(callback.call_count,2)
        self.assertEqual(stage.calls.count('complete_constraint_fallback_probe'),3)
        self.assertEqual(len(calls),8)


if __name__=='__main__':unittest.main()
