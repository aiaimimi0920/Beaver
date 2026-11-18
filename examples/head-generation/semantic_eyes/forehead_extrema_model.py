"""Private LP-only all-child model; original nonlinear residuals stay untouched."""
import numpy as np
from scipy.sparse import coo_matrix
from forehead_extrema_primitives import forehead_extrema_edges, forehead_extrema_turns


class ForeheadExtremaLinearModel:
    __slots__ = ('owners', 'residuals', 'jacobian', 'handled_rows',
                 'unsupported_rows', 'labels', 'aggregate_count')

    def __init__(self, owners, residuals, jacobian, handled_rows,
                 unsupported_rows, labels, aggregate_count):
        self.owners = owners
        self.residuals = residuals
        self.jacobian = jacobian
        self.handled_rows = handled_rows
        self.unsupported_rows = unsupported_rows
        self.labels = labels
        self.aggregate_count = aggregate_count

    def inequalities(self, absolute_parent_goals):
        """Return A,b for A*d <= b using the PARENT's already-computed goals.

        Do not independently apply fractional restoration to child residuals.
        Do not use children to replace the original nonlinear merit or admission.
        Caller retains original trust/bounds, deadline, LP limits and line search.
        """
        goal = np.asarray(absolute_parent_goals, float)
        if goal.shape != (self.aggregate_count,) or not np.isfinite(goal).all():
            raise ValueError('Invalid original parent goals')
        return -self.jacobian, self.residuals - goal[self.owners]


def forehead_role_extrema_model(stage, points, variable_ids, sampled,
        source_differential, source_boundary, offset, scales, aggregate_residuals):
    """Every existing role extreme is the minimum of its child residuals.

    Expands only role rows 1,3,4,6,8,10, not rail/support extrema or energies.
    Units/scales/caps/reference are inherited exactly. Nonsmooth source and
    singular child rows are explicitly returned for the caller's old fallback.
    This object is neither an accepted proposal nor a nonlinear feasibility proof.
    """
    stage.charge('extrema_linear_model')
    p = np.asarray(points, float)
    ids = np.asarray(variable_ids)
    sampled = np.asarray(sampled, float)
    differential = np.asarray(source_differential, float)
    boundary = np.asarray(source_boundary)
    scales = np.asarray(scales, float)
    actual = np.asarray(aggregate_residuals, float)
    if (p.shape != (stage.points_count, 3) or not np.isfinite(p).all()
            or ids.ndim != 1 or ids.dtype.kind not in 'iu'
            or len(set(ids.tolist())) != len(ids) or np.any(ids < 0) or np.any(ids >= len(p))
            or sampled.shape != (len(ids), 3) or not np.array_equal(sampled, p[ids])
            or differential.shape != (len(ids), 3, 2) or not np.isfinite(differential).all()
            or boundary.shape != (len(ids),) or boundary.dtype.kind != 'b'
            or type(offset) is not int or offset < 0 or scales.ndim != 1
            or offset + stage.count > len(scales) or np.any(scales <= 0)
            or not np.isfinite(scales).all() or actual.shape != (stage.count,)
            or not np.isfinite(actual).all()):
        raise ValueError('Invalid all-child model ownership or source differential')
    index = {int(vertex): i for i, vertex in enumerate(ids)}
    boundary_vertices = set(ids[boundary].tolist())
    owners, residuals, labels, rows, columns, data = [], [], [], [], [], []
    handled, unsupported = [], []
    factors = np.maximum(abs(np.asarray(stage.reference)), 1.)

    def append_owner(local_row, vertices, values, gradients, name, invalid=False):
        owner = offset + local_row
        if invalid or boundary_vertices.intersection(vertices.ravel().tolist()):
            unsupported.append(owner)
            return
        denominator = factors[local_row] * scales[owner]
        child = (stage.caps[local_row] - np.asarray(values)) / denominator
        expected = actual[local_row] / scales[owner]
        if not np.isclose(child.min(), expected, rtol=2e-12, atol=1e-10):
            raise ValueError('Child residual minimum differs from original aggregate')
        handled.append(owner)
        for number, (value, verts, world) in enumerate(zip(child, vertices, gradients)):
            row = len(owners)
            owners.append(owner)
            residuals.append(float(value))
            labels.append((name, number))
            for vertex, gradient in zip(verts, world):
                parameter = index.get(int(vertex))
                if parameter is not None:
                    local = -(gradient @ differential[parameter]) / (1000 * denominator)
                    for component in range(2):
                        rows.append(row)
                        columns.append(2 * parameter + component)
                        data.append(float(local[component]))

    for role_index, (role, path) in enumerate(stage.paths.items()):
        path = np.asarray(path, int)
        q = p[path]
        edges = np.asarray(list(zip(path, path[1:])), int)
        triples = np.asarray(list(zip(path, path[1:], path[2:])), int)
        length, length_gradient = forehead_extrema_edges(q)
        base = 11 * role_index
        append_owner(base + 3, edges, -length, -length_gradient, role + '.min_length')
        append_owner(base + 4, edges, length, length_gradient, role + '.max_length')
        angle, gradient, invalid = forehead_extrema_turns(q)
        append_owner(base + 1, triples, angle, gradient, role + '.peak_3d', bool(invalid.any()))
        for frame_index, (name, frame) in enumerate(stage.frames.items()):
            angle, gradient, invalid = forehead_extrema_turns(q, frame)
            append_owner(base + 6 + 2 * frame_index, np.concatenate((triples, triples)),
                         np.r_[angle, -angle], np.concatenate((gradient, -gradient)),
                         role + '.' + name + '_signed_peak', bool(invalid.any()))
    matrix = coo_matrix((data, (rows, columns)), shape=(len(owners), 2 * len(ids))).tocsr()
    matrix.eliminate_zeros()
    if not np.isfinite(matrix.data).all():
        raise ValueError('Nonfinite child derivative')
    return ForeheadExtremaLinearModel(np.asarray(owners, int), np.asarray(residuals), matrix,
                                     tuple(handled), tuple(unsupported), tuple(labels), len(scales))
