"""Exact dyadic candidate-triangle threshold predicate; no nearest-mesh claim."""
from math import gcd
import struct


class IntegerBudgetExceeded(Exception):
    """No certificate produced within the configured integer resource budget."""


def binary64_ratio(value):
    """Decode bits directly; even caller floating-point rounding/FTZ is irrelevant."""
    bits = struct.unpack("<Q", struct.pack("<d", value))[0]
    exponent, numerator = (bits >> 52) & 0x7ff, bits & ((1 << 52) - 1)
    if exponent == 0x7ff:
        raise ValueError("Nonfinite binary64 input")
    power = -1074
    if exponent:
        numerator |= 1 << 52
        power = exponent - 1023 - 52
    if not numerator:
        return 0, 1
    if power < 0:
        trailing = min(-power, (numerator & -numerator).bit_length() - 1)
        numerator >>= trailing
        power += trailing
    numerator = -numerator if bits >> 63 else numerator
    return (numerator << power, 1) if power >= 0 else (numerator, 1 << -power)


def _sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def _dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def _grid(triangle, point, radius, max_integer_bits):
    values = [x for vertex in triangle for x in vertex] + list(point) + [radius]
    ratios = [binary64_ratio(x) for x in values]
    powers = [denominator.bit_length() - 1 for _, denominator in ratios]
    exponent = max(powers)
    integers = [numerator << (exponent - power)
                for (numerator, _), power in zip(ratios, powers)]
    bits = max(1, max(abs(value).bit_length() for value in integers))
    # Coordinates have <=bits bits; differences add one, dot/cross sums add a
    # small constant. Degree <=6 expressions and the physical squared-distance
    # denominator fit these conservative bounds. Check BEFORE multiplying.
    if max(6 * bits + 32, 2 * exponent + 4 * bits + 32) > max_integer_bits:
        raise IntegerBudgetExceeded
    vertices = tuple(tuple(integers[i:i + 3]) for i in (0, 3, 6))
    return vertices, tuple(integers[9:12]), integers[12], exponent


def _proof(feature, numerator, denominator, exponent):
    denominator <<= 2 * exponent
    common = gcd(numerator, denominator)
    return {"feature": feature, "squared_numerator_hex": hex(numerator // common),
            "squared_denominator_hex": hex(denominator // common)}


def candidate_within(triangle, point, radius, max_integer_bits):
    """Validated binary64 inputs only. Return exact witness or None; never FAIL.

    The radius is the represented binary64 value, squared in integer arithmetic,
    NOT a rounded binary64 product. No square root, float predicate or tolerance.
    """
    (a, b, c), p, radius, exponent = _grid(triangle, point, radius, max_integer_bits)
    radius_squared = radius * radius
    for name, start, end in (("AB", a, b), ("BC", b, c), ("CA", c, a)):
        edge, offset = _sub(end, start), _sub(p, start)
        length_squared, along = _dot(edge, edge), _dot(offset, edge)
        numerator, denominator = _dot(offset, offset), 1
        if length_squared and along >= length_squared:
            numerator = _dot(_sub(p, end), _sub(p, end))
        elif length_squared and along > 0:
            numerator = numerator * length_squared - along * along
            denominator = length_squared
        if numerator <= radius_squared * denominator:
            return _proof(name, numerator, denominator, exponent)
    edge, other, offset = _sub(b, a), _sub(c, a), _sub(p, a)
    aa, ab, bb = _dot(edge, edge), _dot(edge, other), _dot(other, other)
    da, db = _dot(offset, edge), _dot(offset, other)
    normal = _cross(edge, other)
    normal_squared = _dot(normal, normal)
    if normal_squared:  # Exact nonzero area, not a tolerance.
        u, v = bb * da - ab * db, aa * db - ab * da
        if u >= 0 and v >= 0 and u + v <= normal_squared:
            height_numerator = _dot(offset, normal)
            numerator = height_numerator * height_numerator
            if numerator <= radius_squared * normal_squared:
                return _proof("INTERIOR", numerator, normal_squared, exponent)
    return None
