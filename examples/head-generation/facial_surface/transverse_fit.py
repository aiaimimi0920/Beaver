"""CPU-only constrained transverse experiment; not a native recipe."""

from math import comb
import numpy as np
from scipy.optimize import lsq_linear


def basis(t, degree):
    return np.stack(
        [comb(degree, i) * t**i * (1 - t) ** (degree - i) for i in range(degree + 1)],
        axis=-1,
    )


def fit_response(regularization):
    if not 0 < regularization <= 0.01:
        raise ValueError("Expected positive bounded regularization")
    t = np.linspace(0, 1, 301)
    matrix = basis(t, 7)
    difference = np.diff(np.eye(8), n=3, axis=0)
    third = basis(t, 4) @ (210 * difference)
    system = np.vstack((matrix, np.sqrt(regularization) * third))
    responses = []
    for center, target in ((1.0, np.ones(len(t))), (0.0, (0.85 * t) ** 2)):
        fixed = np.array([center, center, 0, 0, 0, 0, 0, 0], dtype=float)
        rhs = np.r_[target, np.zeros(len(t))] - system @ fixed
        # Nonnegative bounded Bernstein coefficients provide a continuous hull
        # bound. Boundary coefficient triples are fixed to zero (C2 collar).
        free = lsq_linear(system[:, 2:5], rhs, bounds=(0.0, 1.0), tol=1e-12)
        if not free.success:
            raise ValueError("Transverse fit did not converge")
        fixed[2:5] = free.x
        responses.append(fixed.tolist())
    return dict(
        regularization=regularization,
        axial=responses[0],
        rounding=responses[1],
        domain_q=[0, 0.85],
        degree=7,
        scope="Experimental basis only; full-surface curvature and attachment acceptance pending",
    )


def response(q, coefficients):
    if abs(q) >= 0.85:
        return 0.0
    t = abs(q) / 0.85
    return float(basis(np.asarray(t), 7) @ np.asarray(coefficients))


def correction(q, h, height, front, back, target_front, parameters, fit):
    if abs(q) >= 0.85 or h <= 0.1 or h >= 0.9:
        return 0.0
    from hypothesis import ease

    axial = height * (target_front - front) * parameters["axial_smoothing"]
    transverse = height * (back - front) * 0.72 * parameters["cheek_roundness"]
    transverse *= ease((h - 0.1) / 0.1) * ease((0.9 - h) / 0.1)
    return axial * response(q, fit["axial"]) + transverse * response(q, fit["rounding"])


def continuous_base_bound(calibration, curve, axial_profile_bound_m, parameters, fit):
    """Conservative value bound: exact cubic profile extrema + Bernstein hulls.

    This certifies the base correction only, not orbital displacement or fitted
    component geometry. Height collars are in[0,1]. Floating arithmetic uses a
    conservative 1nm margin; this is not a formal interval-arithmetic proof.
    """
    front = calibration["continuous_front_profile"]
    back = calibration["continuous_back_profile"]
    breaks = sorted({0.1, 0.9, *[p[0] for p in front + back if 0.1 < p[0] < 0.9]})
    maximum = 0.0
    for lo, hi in zip(breaks, breaks[1:]):
        ts = np.linspace(0, hi - lo, 4)
        coeff = np.polynomial.polynomial.polyfit(
            ts, [curve(lo + t, 1, back) - curve(lo + t, 1, front) for t in ts], 3
        )
        candidates = [0.0, hi - lo]
        for root in np.polynomial.polynomial.polyroots(
            np.polynomial.polynomial.polyder(coeff)
        ):
            if abs(root.imag) < 1e-10 and 0 < root.real < hi - lo:
                candidates.append(root.real)
        maximum = max(
            maximum,
            max(abs(np.polynomial.polynomial.polyval(t, coeff)) for t in candidates),
        )
    bound = (
        axial_profile_bound_m
        * parameters["axial_smoothing"]
        * max(abs(x) for x in fit["axial"])
    )
    bound += (
        calibration["height_m"]
        * maximum
        * 0.72
        * parameters["cheek_roundness"]
        * max(abs(x) for x in fit["rounding"])
    )
    return float(bound + 1e-9)
