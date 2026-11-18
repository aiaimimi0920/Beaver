"""Guarded local-shape derivative assembly with finite paired-source probes.

Invalid source probes are errors, never finite -1000 sentinel derivatives.
Original positive/negative scalar constraints and numerical step sizes remain.
This assembler expects the local operator's multi-row residual vector.
"""

import numpy as np


def local_shape_safe_derivative(values, evaluate, base, constraints, batch,
                                field_batch, ids, forward, reverse, base_count,
                                numerical):
    """Assemble using original FD conventions and only consumed analytic columns.

    `numerical` is the existing approx_derivative dependency, injected for tests.
    No solver is invoked. Both poses, primal/differential identity and every
    evaluated residual are checked before they may feed a finite difference.
    """
    values = np.asarray(values)
    ids = np.asarray(ids, dtype=int)
    if values.ndim != 1 or len(values) != 2*len(ids) or not np.isfinite(values).all():
        raise ValueError("Invalid local-shape derivative parameters")
    total_count = base_count + forward.count + reverse.count

    def valid_state(sample):
        state = evaluate(sample)
        if state is None or not isinstance(state, (tuple, list)) or len(state) != 2:
            raise ValueError("Invalid local-shape derivative source probe")
        points, field = (np.asarray(item) for item in state)
        if (points.ndim != 2 or points.shape[1] != 3 or field.shape != points.shape
                or not np.isfinite(points).all() or not np.isfinite(field).all()
                or not np.isfinite(points + field).all()):
            raise ValueError("Nonfinite or malformed local-shape paired source")
        return points, field

    def residual(function, sample, count):
        valid_state(sample)
        result = np.asarray(function(sample))
        if result.shape != (count,) or not np.isfinite(result).all():
            raise ValueError("Nonfinite or malformed local-shape probe residual")
        return result

    def guarded_full(sample):
        return residual(constraints, sample, total_count)

    def full_numerical():
        result = np.asarray(numerical(guarded_full, values, method="2-point", abs_step=1e-5))
        if result.shape != (total_count, len(values)) or not np.isfinite(result).all():
            raise ValueError("Invalid local-shape numerical derivative")
        return result

    points, field = valid_state(values)
    guarded_full(values)
    coordinates = values.reshape(-1, 2) / 1000

    def unpack(result, expected):
        if not isinstance(result, (tuple, list)) or len(result) != 3:
            raise ValueError("Malformed local-shape source differential")
        sampled, derivative, boundary = result
        sampled, boundary = np.asarray(sampled), np.asarray(boundary)
        if (sampled.shape != expected.shape or not np.array_equal(sampled, expected)
                or boundary.shape != (len(ids),) or boundary.dtype.kind != "b"):
            raise ValueError("Local-shape differential/primal or boundary mismatch")
        if derivative is None:
            return None, boundary
        derivative = np.asarray(derivative)
        if derivative.shape != (len(ids), 3, 2):
            raise ValueError("Malformed local-shape source Jacobian")
        return derivative, boundary

    geometry_result = batch.sample_differential(coordinates)
    if geometry_result is None:
        return full_numerical()
    geometry_jacobian, geometry_boundary = unpack(geometry_result, points[ids])
    if geometry_jacobian is None:
        return full_numerical()
    expression_result = field_batch.sample_differential(coordinates)
    if expression_result is None:
        return full_numerical()
    field_jacobian, field_boundary = unpack(expression_result, field[ids])
    if field_jacobian is None:
        return full_numerical()
    boundary = geometry_boundary | field_boundary
    smooth_columns = np.flatnonzero(~np.repeat(boundary, 2))
    boundary_columns = np.flatnonzero(np.repeat(boundary, 2))
    output = np.empty((total_count, len(values)))

    if len(smooth_columns):
        # Keep full ids and matrix shapes, avoiding empty-index issues in the
        # existing forward assembler. Explicit assignment, never NaN * zero.
        geometry_jacobian = geometry_jacobian.copy()
        field_jacobian = field_jacobian.copy()
        geometry_jacobian[boundary] = 0
        field_jacobian[boundary] = 0
        if not np.isfinite(geometry_jacobian).all() or not np.isfinite(field_jacobian).all():
            raise ValueError("Nonfinite consumed local-shape source Jacobian")

        def reduced_base(reduced):
            sample = values.copy()
            sample[smooth_columns] = reduced
            return residual(base, sample, base_count)

        base_jacobian = np.asarray(numerical(reduced_base, values[smooth_columns],
                                            method="2-point", abs_step=1e-5))
        if base_jacobian.shape != (base_count, len(smooth_columns)):
            raise ValueError("Malformed local-shape numerical base derivative")
        first = forward.jacobian(points, field, ids, geometry_jacobian/1000,
                                 field_jacobian/1000).toarray()
        last = reverse.jacobian(points, field, ids, geometry_jacobian/1000,
                                field_jacobian/1000).toarray()
        if first.shape != (forward.count, len(values)) or last.shape != (reverse.count, len(values)):
            raise ValueError("Malformed local-shape analytic derivative")
        output[:, smooth_columns] = np.vstack((base_jacobian,
                                               first[:, smooth_columns], last[:, smooth_columns]))
        if not np.isfinite(output[:, smooth_columns]).all():
            raise ValueError("Nonfinite consumed local-shape analytic derivative")

    # If all variables are boundary flagged, no unused analytic/base-FD path runs.
    for column in boundary_columns:
        plus, minus = values.copy(), values.copy()
        plus[column] += 1e-4
        minus[column] -= 1e-4
        output[:, column] = (guarded_full(plus)-guarded_full(minus)) / (2e-4)
    if not np.isfinite(output).all():
        raise ValueError("Nonfinite local-shape derivative result")
    return output
