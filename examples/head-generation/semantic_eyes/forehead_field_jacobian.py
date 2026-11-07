"""Exact active-donor posed-field norm Jacobian, solver parameters in mm."""
import numpy as np
from scipy import sparse


def forehead_field_parameter_jacobian(delta, differential, frozen, boundary):
    """Return Nx(2N) raw-mm residual derivative and nonsmooth-row flags.

    c_i=budget_mm-1000*norm(delta_i-frozen_i); delta differential is metres
    per author-chart metre, parameters are millimetres: factors1000 cancel.
    At zero norm the field bound is inactive; choose zero norm subgradient.
    Flagged nonzero boundary rows require caller fallback, never silently label
    the selected donor's one-sided derivative a smooth central derivative.
    """
    value=np.asarray(delta,float); D=np.asarray(differential,float)
    base=np.asarray(frozen,float); flags=np.asarray(boundary)
    if (value.ndim!=2 or value.shape[1]!=3 or base.shape!=value.shape or
            D.shape!=(len(value),3,2) or flags.shape!=(len(value),) or flags.dtype.kind!='b' or
            not all(np.isfinite(x).all()for x in (value,D,base))):
        raise ValueError('Invalid field differential inputs')
    diff=value-base;length=np.linalg.norm(diff,axis=1)
    unit=np.divide(diff,length[:,None],out=np.zeros_like(diff),where=length[:,None]>0)
    derivative=-np.einsum('ni,nij->nj',unit,D)
    rows=np.repeat(np.arange(len(value)),2);cols=np.arange(2*len(value))
    J=sparse.csr_matrix((derivative.ravel(),(rows,cols)),shape=(len(value),2*len(value)))
    return J,flags & (length>0)
