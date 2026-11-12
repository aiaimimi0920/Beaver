"""One bounded restoration leg of the SAME frozen fairing problem, not acceptance."""
import time
import numpy as np
from scipy.optimize import Bounds, NonlinearConstraint, minimize
from scipy.sparse import eye


def forehead_restore_fairing_iterate(warm, original_start, radii, constraints,
                                     sparsity, orientation_rows, remember_feasible,
                                     max_iterations=80, jacobian=None, wall_seconds=120.):
    """Call only after fairing produced no admissible checkpoint.

    constraints/Jacobian/scales/IDs/targets must be the SAME captured closures
    from that fit. No proposal, lineage, target, field or reference is rebuilt.
    Movement is tethered to warm, while Bounds retain original_start +/- radii.
    remember_feasible is the original quantized feasibility/best selector; the
    caller MUST still execute its final scalar source-row reconstruction and
    all existing geometry/field/flow/gap gates. Optimizer success is not admission.
    """
    if type(max_iterations) is not int or not 1 <= max_iterations <= 120:
        raise ValueError('Invalid bounded restoration iteration budget')
    if not np.isfinite(wall_seconds) or not 0 < wall_seconds <= 120:
        raise ValueError('Invalid bounded restoration wall budget')
    warm = np.asarray(warm, float).copy()
    anchor = np.asarray(original_start, float).copy()
    radius = np.asarray(radii, float).copy()
    if (warm.ndim != 1 or warm.shape != anchor.shape or warm.shape != radius.shape
            or not len(warm) or not np.isfinite(np.r_[warm, anchor, radius]).all()
            or np.any(radius <= 0)):
        raise ValueError('Invalid frozen parameter domain')
    lower, upper = anchor-radius, anchor+radius
    if np.any(warm < lower) or np.any(warm > upper):
        raise ValueError('Warm iterate exceeds ORIGINAL parameter bounds')
    initial = np.asarray(constraints(warm), float)
    a, b = orientation_rows
    if (initial.ndim != 1 or not np.isfinite(initial).all() or
            not 0 <= a < b <= len(initial) or np.any(initial[a:b] <= 0)):
        raise ValueError('Restoration requires finite rows and positive actual orientation')
    keep = np.zeros(len(initial), dtype=bool)
    keep[a:b] = True
    hard = NonlinearConstraint(constraints, 0., np.inf,
        jac=jacobian if jacobian is not None else '2-point', keep_feasible=keep,
        finite_diff_rel_step=1e-8,
        finite_diff_jac_sparsity=sparsity if jacobian is None else None)
    began = time.monotonic()
    selected = [False]
    timed_out = [False]

    def checkpoint(values, state=None):
        selected[0] = bool(remember_feasible(values)) or selected[0]
        timed_out[0] = time.monotonic()-began >= wall_seconds
        return selected[0] or timed_out[0]

    factor = 2./len(warm)
    solved = minimize(lambda values: np.mean((values-warm)**2), warm,
        jac=lambda values: factor*(values-warm),
        hess=lambda values: eye(len(warm), format='csr')*factor,
        method='trust-constr', bounds=Bounds(lower, upper, keep_feasible=True),
        constraints=[hard], callback=checkpoint,
        options={'maxiter': max_iterations, 'initial_tr_radius': .5,
                 'initial_barrier_parameter': 1e-5, 'initial_barrier_tolerance': 1e-4,
                 'gtol': 1e-7, 'xtol': 1e-10, 'barrier_tol': 1e-10,
                 'sparse_jacobian': True, 'verbose': 1})
    checkpoint(solved.x)
    metadata = {'phase': 'fairing_then_same_problem_restoration',
                'iteration_budget': max_iterations, 'wall_budget_seconds': wall_seconds,
                'elapsed_seconds': time.monotonic()-began,
                'wall_budget_reached': timed_out[0],
                'original_parameter_bounds_retained': True,
                'same_frozen_constraints_and_jacobian': True,
                'callback_selected_feasible': selected[0],
                'full_model_success': False,
                'scope': 'Internal checkpoint only; final scalar and all original admission gates remain mandatory'}
    return solved, metadata


def forehead_minimum_improvement_passes(initial_cost, final_cost, minimum):
    """A positive requested minimum is admission, not merely an early-stop hint."""
    if not 0 <= minimum < 1 or not np.isfinite([initial_cost, final_cost]).all():
        return False
    return bool(final_cost <= (1-minimum)*initial_cost)
