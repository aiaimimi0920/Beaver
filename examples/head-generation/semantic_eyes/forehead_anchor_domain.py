"""Necessary frozen-frame endpoint rows, not sufficient atlas validity guards."""
import numbers
import numpy as np
from scipy import sparse


def make_forehead_anchor_domain(fast, params, ids):
    """Construct source-owned necessary endpoint-domain constraints."""
    return ForeheadAnchorDomain(fast, params, ids)


class ForeheadAnchorDomain:
    """Own immutable anchor references; never freeze interior chart membership.

    Parameters are Nx3 original-author chart rows in meters. Solver variables are
    flat (u,height) pairs in millimeters, in the supplied unique-ID order.
    Full atlas/source/quality/field admission remains external and mandatory.
    """

    def __init__(self, fast, params, ids):
        parameters = np.asarray(params, dtype=float)
        if (parameters.ndim != 2 or parameters.shape[1] != 3
                or len(parameters) < 2 or not np.isfinite(parameters).all()):
            raise ValueError("Anchor parameters must be finite Nx3 chart rows")
        count = len(parameters)

        def checked_indices(values, label):
            result = []
            for value in values:
                if isinstance(value, (bool, np.bool_)) or not isinstance(value, numbers.Integral):
                    raise ValueError(label + " must contain integer indices")
                value = int(value)
                if not 0 <= value < count:
                    raise ValueError(label + " index outside parameter domain")
                result.append(value)
            if len(set(result)) != len(result):
                raise ValueError(label + " contains duplicate indices")
            return result

        self.ids = checked_indices(ids, "Variable IDs")
        self.index = {node: k for k, node in enumerate(self.ids)}
        pivot = np.asarray(fast.pivot, dtype=float)
        height = float(fast.height)
        if pivot.shape != (2,) or not np.isfinite(pivot).all() or not np.isfinite(height):
            raise ValueError("Anchor frame must be finite with a two-coordinate pivot")
        if pivot[1] != height:
            raise ValueError("Frozen pivot height differs from atlas seam")
        if hasattr(fast, "entry") and float(fast.entry) != pivot[0]:
            raise ValueError("Frozen pivot entry differs from atlas entry")
        names = list(fast.names)
        if not names or any(not isinstance(name, str) for name in names) or len(set(names)) != len(names):
            raise ValueError("Anchor roles must be unique nonempty named roles")
        self.fixed = parameters[:, [0, 2]].copy() * 1000
        self.pivot = pivot.copy() * 1000
        if not np.isfinite(self.fixed).all() or not np.isfinite(self.pivot).all():
            raise ValueError("Anchor millimeter conversion is nonfinite")
        self.specs = []
        for name in names:
            if name not in fast.paths or name not in fast.endpoint_chart:
                raise ValueError("Missing path or frozen endpoint for anchor role")
            path = checked_indices(fast.paths[name], "Role path")
            if len(path) < 2:
                raise ValueError("Anchor role path needs at least two vertices")
            if path[0] in self.index or path[-1] in self.index:
                raise ValueError("Anchor endpoints must remain pinned")
            endpoint = np.asarray(fast.endpoint_chart[name], dtype=float)
            if endpoint.shape != (2,) or not np.isfinite(endpoint).all():
                raise ValueError("Frozen endpoint chart must be a finite pair")
            if not np.array_equal(parameters[path[-1], [0, 2]], endpoint):
                raise ValueError("Pinned endpoint differs from frozen author chart")
            if hasattr(fast, "atlas"):
                author_chart = np.asarray(fast.atlas.sc, dtype=float)
                if (author_chart.ndim != 2 or author_chart.shape[1] != 3
                        or max(path[0], path[-1]) >= len(author_chart)):
                    raise ValueError("Anchor endpoints lack original author identities")
                for node in (path[0], path[-1]):
                    if not np.array_equal(parameters[node, [0, 2]], author_chart[node, [0, 2]]):
                        raise ValueError("Pinned first/last chart differs from original author")
            self.specs.append((name, "upper", path[0], path[1]))
            kind = "lower" if endpoint[1] < height - 1e-10 else "radial"
            self.specs.append((name, kind, path[-2], path[-1]))
        start = self.fixed[self.ids].ravel()
        raw, unused = self.raw(start)
        threshold = np.array([
            1e-8 if kind == "radial" else 1e-11 for _, kind, _, _ in self.specs
        ])
        if np.any(raw <= threshold):
            raise ValueError("Accepted anchor-domain source baseline is not strictly valid")
        cap = np.array([.01 if kind == "radial" else .001 for _, kind, _, _ in self.specs])
        self.margin = threshold + np.minimum((raw - threshold) * .5, cap)
        self.initial = raw
        self.threshold = threshold
        for array in (self.fixed, self.pivot, self.margin, self.initial, self.threshold):
            array.setflags(write=False)

    def raw(self, x):
        variables = np.asarray(x, dtype=float)
        if variables.shape != (2 * len(self.ids),) or not np.isfinite(variables).all():
            raise ValueError("Anchor solver vector must be finite, flat and match variable IDs")
        points = self.fixed.copy()
        points[self.ids] = variables.reshape(-1, 2)
        values = []
        jacobian = sparse.lil_matrix((len(self.specs), len(variables)))
        for row, (name, kind, first, last) in enumerate(self.specs):
            if kind == "upper":
                value = points[last, 0] - points[first, 0]
                first_gradient = np.array([-1., 0.])
                last_gradient = -first_gradient
            elif kind == "lower":
                value = points[first, 1] - points[last, 1]
                first_gradient = np.array([0., 1.])
                last_gradient = -first_gradient
            else:
                a = points[first] - self.pivot
                b = points[last] - self.pivot
                value = a[1] * b[0] - a[0] * b[1]
                first_gradient = np.array([-b[1], b[0]])
                last_gradient = np.array([a[1], -a[0]])
            if (not np.isfinite(value) or not np.isfinite(first_gradient).all()
                    or not np.isfinite(last_gradient).all()):
                raise ValueError("Anchor row or gradient is nonfinite")
            values.append(value)
            for node, gradient in ((first, first_gradient), (last, last_gradient)):
                if node in self.index:
                    column = 2 * self.index[node]
                    jacobian[row, column:column+2] = gradient
        return np.asarray(values), jacobian.tocsr()

    def values(self, x):
        return self.raw(x)[0] - self.margin

    def jacobian(self, x):
        return self.raw(x)[1]
