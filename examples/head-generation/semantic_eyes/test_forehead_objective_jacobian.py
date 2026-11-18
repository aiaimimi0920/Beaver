"""Sparse support derivatives retain the actual scalar objective and domain guards."""
import unittest
import numpy as np
from forehead_objective_jacobian import make_forehead_objective_jacobian


class ObjectiveJacobianTests(unittest.TestCase):
    def test_mixed_guide_support_gradient_and_color_reuse(self):
        ids=list(range(8));quads=np.array([[0,1,2,3],[4,5,6,7]])
        start=np.linspace(-.2,.3,16);target=start+.1;weights=np.linspace(.2,10.,16)
        def evaluate(x):
            angles=np.array([90+np.sin(x[:4]),90+np.sin(x[8:12])])
            ratios=np.array([np.exp(.1*x[:8].sum()),np.exp(.1*x[8:].sum())])
            return None,angles,ratios,None,None
        def objective(x):
            _,a,r,_,_=evaluate(x)
            return (np.sum((a-90)**2)/10000+.1*np.sum(np.log(r)**2)
                +np.sum(weights*(x-target)**2)+.02*np.sum((x-start)**2)
                +x[0]**4+np.sin(x[1])**2)/10000
        derivative,info=make_forehead_objective_jacobian(objective,evaluate,start,target,weights,ids,{0},quads)
        x=start+.03;h=np.maximum(abs(x)*1e-8,1e-5);expected=[]
        for k,step in enumerate(h):
            trial=x.copy();trial[k]+=step;expected.append((objective(trial)-objective(x))/step)
        np.testing.assert_allclose(derivative(x),expected,rtol=.003,atol=2e-8)
        self.assertLess(info['support_colors'],info['support_variables'])

    def test_backward_stencil_and_invalid_local_domain(self):
        x=np.zeros(8);target=np.ones(8);weights=np.ones(8)
        def evaluate(p):
            return None if np.any(p>0) else (None,np.full((1,4),90.),np.ones(1),None,None)
        objective=lambda p:(np.sum((p-target)**2)+.02*np.sum(p*p))/10000
        derivative,_=make_forehead_objective_jacobian(objective,evaluate,x,target,weights,list(range(4)),{0},np.array([[0,1,2,3]]))
        np.testing.assert_allclose(derivative(x),np.full(8,-.0002),atol=2e-9)
        invalid=lambda p:None
        derivative,_=make_forehead_objective_jacobian(objective,invalid,x,target,weights,list(range(4)),{0},np.array([[0,1,2,3]]))
        with self.assertRaises(ValueError):derivative(x)


if __name__=='__main__':unittest.main()
