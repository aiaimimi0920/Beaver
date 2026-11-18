"""Constrained whole-guide bending; final model gates remain caller-owned."""
import numpy as np
from scipy.optimize import minimize, Bounds, NonlinearConstraint, BFGS
from forehead_guide_fairing import forehead_guides_fairing, forehead_fairing_parameter_gradient


def make_forehead_fairing_objective(evaluate, batch, ids, paths, reference_mm, start):
    lengths = {name: length/1000 for name, length in reference_mm.items()}
    if set(lengths) != set(paths):
        raise ValueError('Frozen whole-guide references are required for fairing')
    metadata = {'donor_fallback_columns': 0,
                'scope': 'Whole supplied positive paths; actual coordinate pins and front-angle guards remain external'}
    count = len(paths)

    def objective(values):
        state = evaluate(values)
        if state is None:
            raise ValueError('Invalid source sample in fairing objective')
        energy, _, _ = forehead_guides_fairing(state[0], paths, lengths)
        return energy/count + .0001*np.mean((values-start)**2)

    def gradient(values):
        state = evaluate(values)
        differential = batch.sample_differential(values.reshape(-1, 2)/1000)
        if state is None or differential is None:
            raise ValueError('Invalid source differential in fairing objective')
        sampled, world_jacobian, boundary = differential
        if not np.array_equal(sampled, state[0][ids]):
            raise ValueError('Fairing derivative and primal sources differ')
        _, world_gradient, _ = forehead_guides_fairing(state[0], paths, lengths)
        result, flagged = forehead_fairing_parameter_gradient(
            world_gradient, ids, world_jacobian/1000, boundary)
        result = result/count + .0002*(values-start)/len(start)
        # The positional regularizer is analytic even at source-cell boundaries.
        flagged &= np.repeat(np.any(world_gradient[ids] != 0, axis=1), 2)
        for column in np.flatnonzero(flagged):
            plus, minus = values.copy(), values.copy()
            plus[column] += 1e-4
            minus[column] -= 1e-4
            result[column] = (objective(plus)-objective(minus))/2e-4
        metadata['donor_fallback_columns'] += int(flagged.sum())
        return result

    return objective, gradient, metadata


def solve_forehead_fairing(start, radii, constraints, jacobian, orientation_rows,
                           callback, objective, gradient, max_iterations):
    initial = constraints(start)
    orientation = slice(*orientation_rows)
    if not np.isfinite(initial).all() or np.any(initial[orientation] <= 0):
        raise ValueError('Fairing requires a convex initial chart')
    keep = np.zeros(len(initial), bool)
    keep[orientation] = True
    hard = NonlinearConstraint(constraints, 0., np.inf, jac=jacobian, keep_feasible=keep)
    return minimize(objective, start, jac=gradient, hess=BFGS(), method='trust-constr',
        bounds=Bounds(start-radii, start+radii, keep_feasible=True), constraints=[hard],
        callback=lambda values, state: callback(values),
        options={'maxiter': max_iterations, 'initial_tr_radius': .5,
                 'initial_barrier_parameter': 1e-5, 'initial_barrier_tolerance': 1e-4,
                 'gtol': 1e-7, 'xtol': 1e-10, 'barrier_tol': 1e-10,
                 'sparse_jacobian': True, 'verbose': 1})
