"""CPU-only candidate contract. No native builder or geometry acceptance yet."""

import math

BOUNDS = {
    "axial_smoothing": (0.0, 1.0),
    "cheek_roundness": (0.0, 0.15),
    "orbit_shallowing": (0.0, 0.5),
}


def validate(request):
    if not isinstance(request, dict) or set(request) - BOUNDS.keys():
        raise ValueError("Only documented facial-surface controls are supported")
    result = dict.fromkeys(BOUNDS, 0.0)
    for name, value in request.items():
        low, high = BOUNDS[name]
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid facial-surface control: " + name)
        result[name] = float(value)
    return result


def ease(t):
    t = min(1.0, max(0.0, t))
    return t * t * t * (10 + t * (-15 + 6 * t))


def outer_collar(q):
    """Full cheek support through q=.65; C2 zero at the outer q=.85 collar."""
    if abs(q) >= 0.85:
        return 0.0
    if abs(q) <= 0.65:
        return 1.0
    return 1 - ease((abs(q) - 0.65) / 0.20)


def base_correction(q, h, height, front, back, target_front, parameters):
    if abs(q) >= 0.85 or h <= 0.1 or h >= 0.9:
        return 0.0
    q = abs(q)
    # Both additions have zero transverse derivative at the mirror center.
    # The inherited .28*abs(x)/width fold is therefore not silently erased.
    axial = height * (target_front - front) * parameters["axial_smoothing"]
    # Add broad curvature to the high-order component; leave .28*q untouched.
    transverse = (
        height * (back - front) * 0.72 * parameters["cheek_roundness"] * (q * q - q**24)
    )
    # Transverse term needs an independent vertical collar; the fitted axial
    # residual already has boundary jets constrained at .1/.9.
    transverse *= ease((h - 0.1) / 0.10) * ease((0.9 - h) / 0.10)
    return outer_collar(q) * (axial + transverse)


def orbital_support(base_old, rim_old, base_delta, parameters):
    """Return new base/rim; only beta changes their relative gap."""
    raw = base_old + base_delta
    aligned_rim = rim_old + base_delta
    beta = parameters["orbit_shallowing"]
    return raw, aligned_rim + beta * (raw - aligned_rim)
