"""Bounded global proportions, independent of Blender and legacy calibration."""
import math
from collections.abc import Mapping

BOUNDS = {"width": (0.92, 1.08), "height": (0.94, 1.06), "depth": (0.96, 1.04)}
PRESETS = {
    "baseline": {"width": 1.0, "height": 1.0, "depth": 1.0},
    "soft_wide": {"width": 1.06, "height": 0.96, "depth": 1.0},
    "slender": {"width": 0.94, "height": 1.06, "depth": 1.0},
}


def validate(proportions):
    if not isinstance(proportions, Mapping) or set(proportions) - set(BOUNDS):
        raise ValueError("Only width, height and depth proportions are supported")
    result = dict(PRESETS["baseline"])
    for key, value in proportions.items():
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError(f"{key} must be a finite number")
        lo, hi = BOUNDS[key]
        if not math.isfinite(value) or not lo <= value <= hi:
            raise ValueError(f"{key} must lie in [{lo}, {hi}]")
        result[key] = float(value)
    return result


def axes(proportions):
    p = validate(proportions)
    return p["width"], p["depth"], p["height"]


def vector(value):
    value = tuple(value)
    if len(value) != 3 or any(isinstance(x, bool) or not isinstance(x, (int, float))
                              or not math.isfinite(x) for x in value):
        raise ValueError("Expected three finite numeric coordinates")
    return value


def pivot_from_style(style, assembly_offset=-0.021):
    c = style["calibration"]
    return vector((0.0, 0.0, c["origin_y_m"] + c["height_m"] * 0.5 + assembly_offset))


def transform_point(point, proportions, pivot):
    point, pivot, scales = vector(point), vector(pivot), axes(proportions)
    if scales == (1.0, 1.0, 1.0):
        return point
    return tuple(p if s == 1 else c + s * (p - c) for p, c, s in zip(point, pivot, scales))


def inverse_point(point, proportions, pivot):
    point, pivot, scales = vector(point), vector(pivot), axes(proportions)
    return tuple(p if s == 1 else c + (p - c) / s for p, c, s in zip(point, pivot, scales))


def mouth_delta(delta, proportions):
    return tuple(d * s for d, s in zip(vector(delta), axes(proportions)))


def inverse_transpose_normal(normal, proportions):
    normal, scales = vector(normal), axes(proportions)
    length = math.sqrt(sum(n * n for n in normal))
    if length == 0:
        raise ValueError("Zero normal has no direction")
    if scales == (1.0, 1.0, 1.0):
        return normal
    mapped = tuple(n / s for n, s in zip(normal, scales))
    norm = math.sqrt(sum(n * n for n in mapped))
    return tuple(n / norm for n in mapped)


def jacobian(proportions):
    return math.prod(axes(proportions))


def measure_profile(points):
    points = [vector(p) for p in points]
    if not points:
        raise ValueError("Cannot measure an empty profile")
    spans = [max(p[i] for p in points) - min(p[i] for p in points) for i in range(3)]
    return dict(zip(("width", "depth", "height"), spans))
