"""Exact-real float32 angle clearance with a pointwise original-evaluator guard."""
from fractions import Fraction as ForeheadAngleFraction
from math import isqrt as forehead_angle_isqrt
import numpy as np


def forehead_angle_fraction_up(value):
    """Smallest checked binary64 upper enclosure of a nonnegative rational."""
    if value < 0:
        raise ValueError('Negative angle clearance')
    try:
        rounded = float(value)
    except OverflowError as error:
        raise ValueError('Unrepresentable angle clearance') from error
    if not np.isfinite(rounded):
        raise ValueError('Nonfinite angle clearance')
    if ForeheadAngleFraction.from_float(rounded) < value:
        rounded = float(np.nextafter(rounded, np.inf))
    if not np.isfinite(rounded) or ForeheadAngleFraction.from_float(rounded) < value:
        raise ValueError('Unrepresentable angle clearance')
    return rounded


def forehead_angle_sqrt_up(value):
    """Exact rational upper bound on sqrt(value), with 128 fractional bits."""
    if value < 0:
        raise ValueError('Negative squared angle clearance')
    scale = 1 << 128
    numerator = value.numerator * scale * scale
    root = forehead_angle_isqrt(numerator // value.denominator)
    if root * root * value.denominator < numerator:
        root += 1
    return ForeheadAngleFraction(root, scale)


def forehead_angle_binary32_fraction(bits):
    """Decode finite IEEE binary32 bits without subnormal floating arithmetic."""
    exponent, mantissa = (bits >> 23) & 255, bits & 0x7fffff
    if exponent == 255:
        raise ValueError('Nonfinite binary32 encoding')
    significand = mantissa if exponent == 0 else (1 << 23) + mantissa
    power = -149 if exponent == 0 else exponent - 150
    value = ForeheadAngleFraction(significand) * ForeheadAngleFraction(2)**power
    return -value if bits & 0x80000000 else value


def forehead_angle_rounding_radius(value, bits, widened):
    """Verify this conversion's nearest-even cell and exact gradual widening.

    No platform-wide rounding-mode assumption is inferred from a few probes.
    Integer neighbor decoding preserves the half-subnormal spacing 2**-150.
    """
    magnitude = bits & 0x7fffffff
    if magnitude >= 0x7f7fffff:
        raise ValueError('Float32 rounding cell is not finitely bounded')
    rounded = forehead_angle_binary32_fraction(bits)
    if ForeheadAngleFraction.from_float(float(widened)) != rounded:
        raise ValueError('Float32 widening does not preserve gradual underflow')
    if magnitude == 0:
        high = ForeheadAngleFraction(1, 1 << 149)
        low = -high
    elif bits & 0x80000000:
        low = forehead_angle_binary32_fraction(bits + 1)
        high = forehead_angle_binary32_fraction(bits - 1)
    else:
        low = forehead_angle_binary32_fraction(bits - 1)
        high = forehead_angle_binary32_fraction(bits + 1)
    lower, upper = (low + rounded)/2, (rounded + high)/2
    if not lower <= value <= upper or (bits & 1 and value in (lower, upper)):
        raise ValueError('Float32 conversion is not nearest ties-to-even')
    return max(high-rounded, rounded-low)/2


def forehead_float32_angle_bound(corner, smooth_degrees, saved_degrees):
    """Bound this point's angle change, not an old point's future LP candidate.

    Three rows are previous vertex, corner, next vertex. All coordinate error
    radii and edge squared norms are exact binary rationals. asin(r) is bounded
    above by r/sqrt(1-r*r), computed without trigonometric library assumptions.
    The returned certificate also encloses the exact difference between the
    caller's original float64 angle outputs at this point and its float32 save.
    It does not claim a uniform bound on arbitrary libm outputs over a whole box.
    """
    p = np.asarray(corner, dtype=np.float64)
    angles = np.asarray([smooth_degrees, saved_degrees], dtype=np.float64)
    if (p.shape != (3, 3) or not np.isfinite(p).all()
            or not np.isfinite(angles).all() or np.any(angles < 0) or np.any(angles > 180)):
        raise ValueError('Invalid finite angle rounding input')
    with np.errstate(over='ignore', invalid='ignore', under='ignore'):
        q = p.astype(np.float32)
        widened = q.astype(np.float64)
    if not np.isfinite(widened).all():
        raise ValueError('Float32 rounding cell is not finitely bounded')
    bits = q.view(np.uint32)
    exact = [[ForeheadAngleFraction.from_float(float(v)) for v in row] for row in p]
    radii = []
    for vertex in range(3):
        radius = []
        for component in range(3):
            radius.append(forehead_angle_rounding_radius(
                exact[vertex][component], int(bits[vertex, component]),
                widened[vertex, component]))
        radii.append(radius)
    radians = ForeheadAngleFraction(0)
    for endpoint in (0, 2):
        length2 = sum((exact[endpoint][k]-exact[1][k])**2 for k in range(3))
        error2 = sum((radii[endpoint][k]+radii[1][k])**2 for k in range(3))
        if not length2 or error2 >= length2:
            raise ValueError('Float32 angle uncertainty reaches a collapsed edge')
        radians += forehead_angle_sqrt_up(error2/(length2-error2))
    # A decimal truncation strictly below pi, not a rounded libm constant.
    pi_lower = ForeheadAngleFraction(314159265358979323846, 10**20)
    geometry = min(ForeheadAngleFraction(180), radians*180/pi_lower)
    observed = abs(ForeheadAngleFraction.from_float(float(angles[1]))-
                   ForeheadAngleFraction.from_float(float(angles[0])))
    combined = max(geometry, observed)
    return forehead_angle_fraction_up(combined), {
        'geometry_bound_degrees': forehead_angle_fraction_up(geometry),
        'original_evaluator_difference_degrees': forehead_angle_fraction_up(observed),
        'current_candidate_only': True}
