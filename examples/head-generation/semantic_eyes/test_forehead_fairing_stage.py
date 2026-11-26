import copy
import unittest
import numpy as np
from forehead_fairing_stage import ForeheadFairingStage, forehead_current_checkpoint


class FairingStageTests(unittest.TestCase):
    def setUp(self):
        self.p = np.array([[0.,0.,0.],[.01,.002,0.],[.02,-.001,.001],[.03,.002,0.],[.04,0.,0.]])
        self.paths = {'rail': list(range(5))}
        self.lengths = {'rail': 42.}
        self.stage = ForeheadFairingStage(self.p, self.paths, self.lengths)

    def test_stale_checkpoint_is_not_current_admission(self):
        self.assertFalse(forehead_current_checkpoint({}, np.array([1.])))
        self.assertFalse(forehead_current_checkpoint({'values': np.array([1.])}, np.array([2.])))
        self.assertTrue(forehead_current_checkpoint({'values': np.array([1.])}, np.array([1.])))

    def test_only_explicit_violating_role_progresses(self):
        stage=ForeheadFairingStage(self.p,self.paths,self.lengths,
            turn_reduction_degrees=.5,turn_goals={'rail':20.},turn_roles=[])
        np.testing.assert_array_equal(stage.constraints(self.p)[:-1],0.)
        stage=ForeheadFairingStage(self.p,self.paths,self.lengths,
            turn_reduction_degrees=.5,turn_goals={'rail':20.},turn_roles=['rail'])
        self.assertTrue(np.all(stage.constraints(self.p)[:-1] <= 0))
        self.assertTrue(np.all(stage.constraints(self.p)[:-1] >= -.500000001))

    def test_invalid_vertex_ownership(self):
        for path in ([0,1,-1], [0,1,99], [0,1,1], [0.,1.,2.], [0,1]):
            with self.assertRaises(ValueError):ForeheadFairingStage(self.p,{'rail':path},self.lengths)

    def test_gradient(self):
        _, g = self.stage.energy_gradient(self.p)
        for i in range(5):
            for j in range(3):
                a,b=self.p.copy(),self.p.copy();a[i,j]+=1e-8;b[i,j]-=1e-8
                fd=(self.stage.energy_gradient(a)[0]-self.stage.energy_gradient(b)[0])/2e-8
                self.assertAlmostEqual(fd,g[i,j],delta=2e-5)

    def test_persistent_reference(self):
        changed=self.p.copy();changed[1,1]*=.8
        again=ForeheadFairingStage(changed,self.paths,self.lengths,self.stage.reference)
        self.assertEqual(again.reference,self.stage.reference)
        self.assertEqual(again.energy_gradient(changed)[0],self.stage.energy_gradient(changed)[0])
        self.assertEqual(again.target_energy,.999*self.stage.energy_gradient(changed)[0])

    def test_exact_turn_nonregression_and_reduction(self):
        self.assertFalse(self.stage.accepts(self.p))
        self.assertEqual(self.stage.constraints(self.p)[-1],self.stage.target_energy-self.stage.initial_energy)
        np.testing.assert_array_equal(self.stage.constraints(self.p)[:-1],0.)

    def test_reference_not_mutated(self):
        ref=copy.deepcopy(self.stage.reference)
        again=ForeheadFairingStage(self.p,self.paths,self.lengths,ref)
        again.reference['terms']['rail']['weight']=2
        self.assertEqual(ref,self.stage.reference)

    def test_topology_and_lengths_fail_closed(self):
        with self.assertRaises(ValueError):
            ForeheadFairingStage(self.p,{'rail':[0,2,1,3,4]},self.lengths,self.stage.reference)
        with self.assertRaises(ValueError):
            ForeheadFairingStage(self.p,self.paths,{'rail':43.},self.stage.reference)

    def test_straight_uniform_zero(self):
        p=self.p.copy();p[:,1:]=0
        stage=ForeheadFairingStage(p,self.paths,self.lengths)
        self.assertEqual(stage.energy_gradient(p)[0],0.)
        np.testing.assert_array_equal(stage.energy_gradient(p)[1],0.)

    def test_invalid_weights_and_target(self):
        for kw in ({'role_weights':{'rail':0}}, {'reduction':0}, {'turn_reduction_degrees':-1}):
            with self.assertRaises(ValueError):ForeheadFairingStage(self.p,self.paths,self.lengths,**kw)

    def test_source_chain(self):
        stage=self.stage; p=self.p; ids=list(range(5));x=p[:,:2].ravel()*1000
        def evaluate(v):
            q=p.copy();q[:,:2]=v.reshape(-1,2)/1000
            return (q,)
        class Batch:
            def sample_differential(self,v):
                q=p.copy();q[:,:2]=v
                J=np.tile(np.array([[1.,0.],[0.,1.],[0.,0.]]),(5,1,1))
                return q,J,np.zeros(5,bool)
        g=stage.energy_parameter_gradient(x,evaluate,Batch(),ids)
        for i in range(len(x)):
            a,b=x.copy(),x.copy();a[i]+=1e-5;b[i]-=1e-5
            fd=(stage.energy_gradient(evaluate(a)[0])[0]-stage.energy_gradient(evaluate(b)[0])[0])/2e-5
            self.assertAlmostEqual(fd,g[i],delta=1e-7)

if __name__=='__main__':unittest.main()
