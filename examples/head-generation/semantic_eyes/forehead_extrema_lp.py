"""LP-only child rows; original aggregate constraints/Jacobian remain authoritative."""
import numpy as np
from scipy import sparse
from forehead_extrema_model import forehead_role_extrema_model


def forehead_extrema_rows(aggregate, jacobian, model):
    """Replace explicitly supported parents, retain every other original row.

    Returned owners map each LP row to its ORIGINAL parent. In particular a
    fixed child is not dropped, and singular/source-boundary parents stay on the
    original finite-difference path. This is not a nonlinear acceptance test.
    """
    c = np.asarray(aggregate, float)
    J = sparse.csr_matrix(jacobian)
    if c.ndim != 1 or J.shape[0] != len(c) or not np.isfinite(c).all() or not np.isfinite(J.data).all():
        raise ValueError('Invalid original all-child row frame')
    if model is None:
        return c, J, np.arange(len(c))
    owners = np.asarray(model.owners)
    values = np.asarray(model.residuals, float)
    children = sparse.csr_matrix(model.jacobian)
    handled = np.asarray(model.handled_rows)
    unsupported = np.asarray(model.unsupported_rows)
    if (model.aggregate_count != len(c) or owners.ndim != 1 or owners.dtype.kind not in 'iu'
            or values.shape != owners.shape or children.shape != (len(owners), J.shape[1])
            or not np.isfinite(values).all() or not np.isfinite(children.data).all()
            or handled.ndim != 1 or unsupported.ndim != 1
            or (len(handled) and handled.dtype.kind not in 'iu')
            or (len(unsupported) and unsupported.dtype.kind not in 'iu')
            or np.any(owners < 0) or np.any(owners >= len(c))
            or np.any(handled < 0) or np.any(handled >= len(c))
            or np.any(unsupported < 0) or np.any(unsupported >= len(c))
            or len(set(handled.tolist())) != len(handled)
            or len(set(unsupported.tolist())) != len(unsupported)
            or set(handled.tolist()) != set(owners.tolist())
            or set(handled.tolist()).intersection(unsupported.tolist())):
        raise ValueError('Invalid all-child row ownership')
    for row in handled:
        if not np.isclose(values[owners == row].min(), c[row], rtol=2e-12, atol=1e-10):
            raise ValueError('Child residual minimum differs from original LP aggregate')
    keep = np.ones(len(c), bool)
    keep[handled.astype(int)] = False
    retained = np.flatnonzero(keep)
    return (np.r_[c[retained], values], sparse.vstack((J[retained], children), format='csr'),
            np.r_[retained, owners])


def forehead_extrema_rhs(aggregate, values, owners, parent_rhs):
    """Broadcast one parent's absolute goal, NOT each child's fractional deficit.

    parent_rhs = aggregate - absolute_goal; hence child - aggregate + parent_rhs
    equals child - that SAME goal. Untouched rows retain the old RHS exactly.
    """
    return values - aggregate[owners] + parent_rhs[owners]


def forehead_extrema_prediction(count, values, rows, owners, step):
    """Return parent-sized affine predictions for the original SOC goal policy."""
    predicted = np.full(count, np.inf)
    np.minimum.at(predicted, owners, values + rows @ step)
    if not np.isfinite(predicted).all():
        raise ValueError('Missing or nonfinite all-child parent prediction')
    return predicted


def forehead_make_extrema_model(stage, evaluate, batch, ids, interval, scales):
    """Build each model at its exact LP base point with shared stage accounting.

    No memoization, no cap/reference reset, no FD removal. The original aggregate
    Jacobian still feeds every margin/scaling/cache consumer unchanged.
    """
    if stage is None:
        return None
    first, last = interval
    if last-first != stage.count:
        raise ValueError('Invalid complete-guide all-child interval')

    def linear_model(values):
        state = evaluate(values)
        if state is None:
            raise ValueError('All-child model outside source')
        differential = batch.sample_differential(np.asarray(values).reshape(-1, 2)/1000)
        if differential is None:
            raise ValueError('Missing all-child source differential')
        sampled, derivative, boundary = differential
        return forehead_role_extrema_model(stage, state[0], ids, sampled, derivative,
            boundary, first, scales, stage.constraints(state[0]))

    return linear_model
