"""Exact nearest-triangle distance Jacobians for fitting, not shape acceptance."""
import numpy as np
from scipy.sparse import coo_matrix

def make_forehead_shape_analytic_samples(*args, **kwargs):
    return ForeheadAnalyticShapeSamples(*args, **kwargs)

class ForeheadAnalyticShapeSamples:
    """Residual/Jacobian mixin; selected-export subclass supplies sample arrays."""
    def values(self,points,opened_delta):
        key=(np.asarray(points).tobytes(),np.asarray(opened_delta).tobytes())
        if key in self.memo:return self.memo[key]
        values=[];gradients=[]
        for cached,pose in zip(self.fields,(points,points+opened_delta)):
            query=np.einsum('ij,ijk->ik',self.weights,pose[self.quads[self.face_ids]])
            distance,_,hits=cached.field.distance(query,details=True)
            values.append(getattr(self,'solve_limit_mm',self.limit_mm)**2-(distance*1000)**2)
            gradients.append(-2e6*(query-hits))
        out=(np.concatenate(values),gradients);self.memo[key]=out
        if len(self.memo)>3:self.memo.pop(next(iter(self.memo)))
        return out
    def constraints(self,points,opened_delta):return self.values(points,opened_delta)[0]
    def jacobian(self,points,opened_delta,ids,world_jac,open_jac):
        _,grads=self.values(points,opened_delta);index={v:k for k,v in enumerate(ids)}
        n=len(self.weights);rows=[];cols=[];data=[];nodes=self.quads[self.face_ids]
        for pose,grad in enumerate(grads):
            derivatives=world_jac if pose==0 else world_jac+open_jac
            for corner in range(4):
                active=np.array([i in index for i in nodes[:,corner]])
                rr=np.flatnonzero(active);local=np.array([index[i]for i in nodes[active,corner]])
                g=np.einsum('ij,ijk->ik',grad[active],derivatives[local])*self.weights[active,corner,None]
                for axis in range(2):rows.extend(rr+pose*n);cols.extend(2*local+axis);data.extend(g[:,axis])
        return coo_matrix((data,(rows,cols)),shape=(2*len(self.weights),2*len(ids))).tocsr()
