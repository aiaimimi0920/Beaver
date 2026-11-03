"""Counted source-owned differential adapter; invalid probes fail closed."""
import numpy as np
from forehead_complete_world_differential import forehead_cd_objective, forehead_cd_chain
from forehead_complete_constraint_differential import forehead_cd_parameter_constraints


class ForeheadCompleteCountedBatch:
    def __init__(self, stage, batch, role):
        self.stage, self.batch, self.role = stage, batch, role

    def sample(self, coordinates):
        self.stage.charge(self.role+'_source_primal')
        result = self.batch.sample(coordinates)
        if result is None or not np.isfinite(result).all():
            raise ValueError('Invalid complete-guide source primal')
        return result

    def sample_differential(self, coordinates):
        self.stage.charge(self.role+'_source_differential')
        result = self.batch.sample_differential(coordinates)
        if result is None or any(not np.isfinite(x).all() for x in result):
            raise ValueError('Invalid complete-guide source differential')
        return result

    def __getattr__(self, name):
        return getattr(self.batch, name)


def forehead_complete_counted_batch(stage, batch, role):
    return batch if stage is None else ForeheadCompleteCountedBatch(stage, batch, role)


def forehead_complete_differential_state(stage, values, evaluate, batch, ids):
    stage.charge('complete_differential_state_wrapper')
    state = evaluate(values)
    if state is None or not np.isfinite(state[0]).all():
        raise ValueError('Invalid complete-guide differential source')
    # Batch proxy accounts the actual differential call, including failure.
    sampled, derivative, boundary = batch.sample_differential(np.asarray(values).reshape(-1, 2)/1000)
    if derivative.shape != (len(ids), 3, 2) or np.asarray(boundary).shape != (len(ids),):
        raise ValueError('Invalid complete-guide differential dimensions')
    if not np.array_equal(sampled, state[0][ids]):
        raise ValueError('Complete-guide derivative/primal source mismatch')
    return state[0], derivative, boundary


def forehead_complete_analytic_objective(stage, values, evaluate, batch, ids):
    stage.charge('complete_objective_differential_wrapper')
    points, differential, boundary = forehead_complete_differential_state(stage, values, evaluate, batch, ids)
    stage.charge('complete_world_objective_assembly')
    actual, world, flagged, detail = forehead_cd_objective(stage, points)
    primal = detail.get('analytic_primal', float('nan'))
    if not np.isfinite(primal) or not np.isfinite(actual) or not np.isclose(primal, actual, rtol=2e-12, atol=1e-10):
        raise ValueError('Complete-guide objective differential/primal mismatch')
    gradient, fallback = forehead_cd_chain(world, ids, differential/1000, flagged, boundary)
    for col in np.flatnonzero(fallback):
        a, b = np.asarray(values).copy(), np.asarray(values).copy()
        a[col] += 1e-4
        b[col] -= 1e-4
        probes = []
        for sample in (a, b):
            stage.charge('complete_objective_fallback_probe')
            state = evaluate(sample)
            if state is None:
                raise ValueError('Complete-guide objective probe outside source')
            probes.append(stage.objective(state[0]))
        gradient[col] = (probes[0]-probes[1])/2e-4
    if not np.isfinite(gradient).all():
        raise ValueError('Nonfinite complete-guide objective differential')
    return gradient


def forehead_complete_analytic_constraints(stage, values, evaluate, batch, ids):
    stage.charge('complete_constraint_differential_wrapper')
    points, differential, boundary = forehead_complete_differential_state(stage, values, evaluate, batch, ids)
    stage.charge('complete_constraint_world_assembly')
    result = forehead_cd_parameter_constraints(stage, points, ids, differential, boundary)
    actual, jacobian, fallback, dependencies, _ = result
    if actual.shape != (stage.count,) or jacobian.shape != (stage.count, len(values)) or fallback.shape != jacobian.shape or dependencies.shape != jacobian.shape:
        raise ValueError('Invalid complete-guide differential block')
    if not np.isfinite(actual).all() or not np.isfinite(jacobian).all() or np.any(fallback & ~dependencies):
        raise ValueError('Invalid complete-guide differential incidence')
    return result
