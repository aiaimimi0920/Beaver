"""Pure runtime evaluation of retained piecewise-cubic front-profile fits."""

import math


def strength(value):
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(value)
        or not 0 <= value <= 1
    ):
        raise ValueError("surface strength must be finite numeric 0..1")
    return float(value)


def evaluate(h, segments, derivative=0):
    if derivative not in (0, 1, 2):
        raise ValueError("Unsupported derivative")
    row = next((r for r in segments if h <= r[1]), segments[-1])
    t = h - row[0]
    a, b, c, d = row[2:]
    return (
        ((a * t + b) * t + c) * t + d
        if derivative == 0
        else ((3 * a * t + 2 * b) * t + c if derivative == 1 else 6 * a * t + 2 * b)
    )


def delta(x, y, calibration, curve, fit, amount):
    if amount == 0:
        return 0.0
    c = calibration
    h = (y - c["origin_y_m"]) / c["height_m"]
    if h <= 0.1 or h >= 0.9:
        return 0.0
    q = abs(x) / (c["height_m"] * curve(h, 1, c["width_stations"]))
    # Global front-to-ear collar only; no internal eye/nose/oral masks. At .85
    # this polynomial and its first two derivatives vanish exactly.
    if q >= 0.85:
        return 0.0
    collar = (1 - (q / 0.85) ** 2) ** 3
    old = curve(h, 1, c["continuous_front_profile"])
    return amount * c["height_m"] * (evaluate(h, fit["segments"]) - old) * collar


def eye_plane_delta(x, y, c, curve, fit, amount):
    """Common tangent plane for iris, pupil and catchlights; no disk warping."""
    cx, cy = (
        c["height_m"] * c["iris_center_x"],
        c["origin_y_m"] + c["height_m"] * c["iris_center_y"],
    )
    if abs(abs(x) - cx) > 0.04 + 1e-9 or abs(y - cy) > 0.04 + 1e-9:
        raise ValueError("Eye sample outside certified local frame")
    epsilon = 1e-5
    center = delta(cx, cy, c, curve, fit, amount)
    dx = (
        delta(cx + epsilon, cy, c, curve, fit, amount)
        - delta(cx - epsilon, cy, c, curve, fit, amount)
    ) / (2 * epsilon)
    dy = (
        delta(cx, cy + epsilon, c, curve, fit, amount)
        - delta(cx, cy - epsilon, c, curve, fit, amount)
    ) / (2 * epsilon)
    return center + (abs(x) - cx) * dx + (y - cy) * dy
