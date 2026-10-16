"""CPU packaging-time constrained cubic fit; SciPy is not needed by Blender."""

import ast
import math
import numpy as np
from scipy.interpolate import BSpline, PPoly
from scipy.linalg import null_space


def inherited_curve(source):
    node = next(
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == "curve"
    )
    scope = {}
    exec(
        compile(ast.Module(body=[node], type_ignores=[]), "<inherited curve>", "exec"),
        scope,
    )
    return scope["curve"]


def fit_profile(source, calibration):
    curve = inherited_curve(source)
    points = calibration["continuous_front_profile"]
    h = np.linspace(0.1, 0.9, 801)
    values = np.array([curve(v, 1, points) for v in h])
    knots = [0.1] * 4 + [0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8] + [0.9] * 4
    count = len(knots) - 4
    basis = BSpline(knots, np.eye(count), 3)
    constraints = np.vstack([basis(x, nu=d) for x in (0.1, 0.9) for d in range(3)])

    # Exact polynomial jets from the inherited cubic segment immediately inside
    # the fitting interval. Finite differentiation is not used for boundary jets.
    def jet(x, order):
        neighbor = x + (1e-6 if x == 0.1 else -1e-6)
        i = next(
            i
            for i in range(len(points) - 1)
            if points[i][0] <= neighbor <= points[i + 1][0]
        )
        lo, hi = points[i][0], points[i + 1][0]
        ts = np.linspace(0, hi - lo, 4)
        coef = np.polynomial.polynomial.polyfit(
            ts, [curve(lo + t, 1, points) for t in ts], 3
        )
        return np.polynomial.polynomial.polyval(
            x - lo, np.polynomial.polynomial.polyder(coef, order)
        )

    boundary = np.array([jet(x, d) for x in (0.1, 0.9) for d in range(3)])
    particular = np.linalg.lstsq(constraints, boundary, rcond=None)[0]
    free = null_space(constraints)
    coefficients = (
        particular
        + free
        @ np.linalg.lstsq(basis(h) @ free, values - basis(h) @ particular, rcond=None)[
            0
        ]
    )
    pp = PPoly.from_spline((knots, coefficients, 3))
    segments = [
        [float(pp.x[i]), float(pp.x[i + 1]), *map(float, pp.c[:, i])]
        for i in range(len(pp.x) - 1)
        if pp.x[i + 1] > pp.x[i]
    ]
    # The difference is piecewise cubic. Its extrema occur at union boundaries
    # or real roots of the quadratic derivative, not only sampled grid points.
    breaks = sorted(
        set(
            [0.1, 0.9]
            + [r[0] for r in segments]
            + [p[0] for p in points if 0.1 < p[0] < 0.9]
        )
    )
    maximum = 0.0
    for lo, hi in zip(breaks, breaks[1:]):
        ts = np.linspace(0, hi - lo, 4)
        polynomial = np.polynomial.polynomial.polyfit(
            ts, [float(pp(lo + t)) - curve(lo + t, 1, points) for t in ts], 3
        )
        candidates = [0.0, hi - lo]
        for root in np.polynomial.polynomial.polyroots(
            np.polynomial.polynomial.polyder(polynomial)
        ):
            if abs(root.imag) < 1e-10 and 0 < root.real < hi - lo:
                candidates.append(root.real)
        maximum = max(
            maximum,
            max(
                abs(np.polynomial.polynomial.polyval(t, polynomial)) for t in candidates
            ),
        )
    bound = float(maximum * calibration["height_m"])
    if not math.isfinite(bound) or bound > 0.002 - 1e-9:
        raise ValueError("Certified profile displacement exceeds 2mm")
    error = float(max(abs(constraints @ coefficients - boundary)))
    if error > 1e-8:
        raise ValueError("Boundary jet constraint failed")
    return dict(
        segments=segments,
        certified_max_profile_offset_m=bound,
        boundary_jet_max_error=error,
        bound_method="piecewise cubic difference critical points; 1nm margin",
        continuity="C2 spline interior; inherited C1 width/outer regions retained",
    )
