"""Optional NumPy-only positive-edge envelope linearization, never admission.

Explicitly import this module; the standard-library package does not import it.
Saved geometry supplies intercepts; exact geometry/Jacobians supply derivatives.
All rows are signed slack (feasible >= 0): use A=-jacobian, b=rhs for A step<=b.
A feasible linear step is NOT proof of nonlinear or full-model feasibility.
"""

import numpy as np


def _real_array(value, name):
    raw = np.asarray(value)
    if raw.dtype.kind not in "fiu":
        raise ValueError(f"{name} must contain real integer or floating values")
    try:
        with np.errstate(over="raise", invalid="raise"):
            result = raw.astype(float)
    except FloatingPointError as exc:
        raise ValueError(f"{name} overflows float64 conversion") from exc
    _finite(result)
    return result


def _nonnegative_scalar(value, name, minimum=0):
    raw = np.asarray(value)
    if raw.ndim != 0 or raw.dtype.kind not in "fiu":
        raise ValueError(f"{name} must be a real scalar")
    result = float(raw)
    if not np.isfinite(result) or result < minimum:
        raise ValueError(f"{name} must be finite and >= {minimum}")
    return result


def _indices(raw, size, name):
    if raw.size and raw.dtype.kind not in "iu":
        raise ValueError(f"{name} must contain integer indices")
    # Check unsigned values before converting to platform signed integers.
    if np.any(raw < 0) or np.any(raw >= size):
        raise ValueError(f"{name} index out of range")
    return raw.astype(np.intp)


def _finite(*arrays):
    if not all(np.all(np.isfinite(array)) for array in arrays):
        raise ValueError("geometry, derivatives and derived values must be finite")


def edge_length_envelope_linearization(
    p_saved,
    p_exact,
    world_J,
    edges,
    ratio_cap,
    relative_cap,
    active_columns=None,
    reserve_floor=1e-7,
    activity_tolerance=1e-12,
):
    """Return every ordered-pair ratio and both per-edge mean-relative row.

    Ratio rows exactly cover max(length)/min(length); relative rows exactly
    cover max(abs(length/mean(length)-1)). The mean derivative uses EVERY edge.
    No row is dropped or clipped, even when inactive with negative slack.

    ``active`` means first-order activity in selected columns only. A zero
    derivative (including at a stationary point) is NOT structural invariance.
    Callers must prove structural invariance separately and recheck nonlinear
    geometry after every step, including first-order-inactive constraints.
    All Jacobian columns are returned even when only some determine activity.

    Active rows reserve max(8*abs(saved_slack-exact_slack)+1e-10, reserve_floor).
    Other rows reserve zero. This explicit rounding heuristic is not an error
    bound or permission to change caller-owned acceptance limits. Inputs and
    all derived values must be finite, with >=2 strictly positive edges.
    """
    ps = _real_array(p_saved, "p_saved")
    pe = _real_array(p_exact, "p_exact")
    w = _real_array(world_J, "world_J")
    raw = np.asarray(edges)
    if ps.ndim != 2 or ps.shape != pe.shape or ps.shape[1] != 3:
        raise ValueError("points must have equal (vertices,3) shape")
    if w.ndim != 3 or w.shape[:2] != pe.shape or w.shape[2] < 1:
        raise ValueError("world_J must have shape (vertices,3,parameters)")
    if raw.ndim != 2 or raw.shape[1] != 2 or len(raw) < 2:
        raise ValueError("edges must be an integer (edges>=2,2) array")
    e = _indices(raw, len(pe), "edges")
    _finite(ps, pe, w)
    ratio_cap = _nonnegative_scalar(ratio_cap, "ratio_cap", 1)
    relative_cap = _nonnegative_scalar(relative_cap, "relative_cap")
    reserve_floor = _nonnegative_scalar(reserve_floor, "reserve_floor")
    activity_tolerance = _nonnegative_scalar(activity_tolerance, "activity_tolerance")
    cols = np.arange(w.shape[2]) if active_columns is None else np.asarray(active_columns)
    if cols.ndim != 1:
        raise ValueError("active_columns must be one-dimensional")
    cols = _indices(cols, w.shape[2], "active_columns")
    try:
        # Trap overflow/invalid arithmetic; zero computed lengths fail below.
        # This does not certify freedom from derivative underflow or rounding.
        with np.errstate(over="raise", divide="raise", invalid="raise"):
            return _linearize(ps, pe, w, e, ratio_cap, relative_cap, cols,
                              reserve_floor, activity_tolerance)
    except FloatingPointError as exc:
        raise ValueError("edge envelope arithmetic overflow or invalid operation") from exc


def _linearize(ps, pe, w, e, ratio_cap, relative_cap, cols, floor, tolerance):
    es = ps[e[:, 1]] - ps[e[:, 0]]
    ex = pe[e[:, 1]] - pe[e[:, 0]]
    ls = np.linalg.norm(es, axis=1)
    le = np.linalg.norm(ex, axis=1)
    _finite(ls, le)
    if np.any(ls <= 0) or np.any(le <= 0):
        raise ValueError("all saved and exact edge lengths must be positive")
    dl = np.einsum("ei,ein->en", ex / le[:, None], w[e[:, 1]] - w[e[:, 0]])
    ms, me = ls.mean(), le.mean()
    dm = dl.mean(axis=0)
    saved, exact, jac, labels = [], [], [], []
    for i in range(len(e)):
        for j in range(len(e)):
            if i == j:
                continue
            saved.append(ratio_cap - ls[i] / ls[j])
            exact.append(ratio_cap - le[i] / le[j])
            jac.append(-(dl[i] - (le[i] / le[j]) * dl[j]) / le[j])
            labels.append(("ratio", i, j))
        grad = (dl[i] - (le[i] / me) * dm) / me
        saved.extend([relative_cap - (ls[i] / ms - 1),
                      relative_cap - (1 - ls[i] / ms)])
        exact.extend([relative_cap - (le[i] / me - 1),
                      relative_cap - (1 - le[i] / me)])
        jac.extend([-grad, grad])
        labels.extend([("relative_upper", i), ("relative_lower", i)])
    saved, exact, jac = np.asarray(saved), np.asarray(exact), np.asarray(jac)
    _finite(saved, exact, jac, dl, dm)
    active = np.any(np.abs(jac[:, cols]) > tolerance, axis=1)
    rounding_delta = np.abs(saved - exact)
    reserve = np.where(active, np.maximum(8 * rounding_delta + 1e-10, floor), 0.)
    rhs = saved - reserve
    _finite(rounding_delta, reserve, rhs)
    return dict(slack=saved, exact_slack=exact, jacobian=jac, labels=labels,
                rounding_delta=rounding_delta, reserve=reserve, active=active, rhs=rhs)
