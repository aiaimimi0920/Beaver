"""Sparse feasibility restoration that never accepts a folded intermediate chart."""
import numpy as np
from scipy.optimize import Bounds, NonlinearConstraint, minimize, BFGS
from scipy.sparse import eye


def restore_forehead_feasibility(start, radii, constraints, sparsity, orientation_rows,
                                 field_rows, callback, max_iterations, jacobian=None):
    initial = constraints(start)
    keep = np.zeros(len(initial), dtype=bool)
    orientation = slice(*orientation_rows)
    if not np.isfinite(initial).all() or np.any(initial[orientation] <= 0):
        raise ValueError('Feasibility restoration requires an initially convex chart')
    keep[orientation] = True
    # Field inequalities remain in this constraint with exactly the same bounds.
    # Do not force their interior-point slack to tiny positive roundoff at an
    # active field limit: mu/slack then dominates and can freeze every variable.
    # Only orientation is kept feasible at every unpublished restoration iterate;
    # the caller still requires strict raw and saved-float32 field/quality guards.
    constraint = NonlinearConstraint(
        constraints, 0., np.inf, jac=jacobian if jacobian is not None else '2-point', keep_feasible=keep,
        finite_diff_rel_step=1e-8, finite_diff_jac_sparsity=sparsity if jacobian is None else None)
    # This phase deliberately has no appearance or equal-spacing objective.
    # First obtain a valid mesh near the original convex initialization.
    factor = 2. / len(start)
    hessian = eye(len(start), format='csr') * factor
    return minimize(
        lambda values: np.mean((values-start)**2), start,
        jac=lambda values: factor*(values-start), hess=lambda values: hessian,
        method='trust-constr', bounds=Bounds(start-radii, start+radii, keep_feasible=True),
        constraints=[constraint], callback=lambda values, state: callback(values),
        options={'maxiter': max_iterations, 'initial_tr_radius': .5,
                 'initial_barrier_parameter': 1e-5, 'initial_barrier_tolerance': 1e-4,
                 'gtol': 1e-7, 'xtol': 1e-10, 'barrier_tol': 1e-10,
                 'sparse_jacobian': True, 'verbose': 1})


def fit_spacing_sparse(start, radii, objective, constraints, sparsity, callback, max_iterations, jacobian=None):
    initial = constraints(start)
    if not np.isfinite(initial).all() or np.any(initial < 0):
        raise ValueError('Sparse spacing requires a restored feasible start')
    hard = NonlinearConstraint(constraints,0.,np.inf,jac='2-point',
        keep_feasible=True,finite_diff_rel_step=1e-8,finite_diff_jac_sparsity=sparsity)
    return minimize(objective,start,method='trust-constr',jac=jacobian if jacobian is not None else '2-point',hess=BFGS(),
        constraints=[hard],bounds=Bounds(start-radii,start+radii,keep_feasible=True),
        callback=lambda values,state:callback(values),
        options={'maxiter':max_iterations,'initial_tr_radius':.5,
                 'initial_barrier_parameter':1e-5,'initial_barrier_tolerance':1e-4,
                 'gtol':1e-7,'xtol':1e-10,'barrier_tol':1e-10,
                 'finite_diff_rel_step':1e-8,'sparse_jacobian':True,'verbose':1})
