"""Retained pure-Python field used by the experimental recipe, no fitting at runtime."""

from math import comb


def evaluate(h, segments):
    row = next((r for r in segments if h <= r[1]), segments[-1])
    t = h - row[0]
    a, b, c, d = row[2:]
    return ((a * t + b) * t + c) * t + d


def response(q, coefficients):
    if abs(q) >= 0.85:
        return 0.0
    t = abs(q) / 0.85
    return sum(
        comb(7, i) * t**i * (1 - t) ** (7 - i) * v for i, v in enumerate(coefficients)
    )


def ease(t):
    t = min(1.0, max(0.0, t))
    return t * t * t * (10 + t * (-15 + 6 * t))


def base_delta(x, y, c, curve, data):
    p = data["parameters"]
    h = (y - c["origin_y_m"]) / c["height_m"]
    if h <= 0.1 or h >= 0.9:
        return 0.0
    q = abs(x) / (c["height_m"] * curve(h, 1, c["width_stations"]))
    front = curve(h, 1, c["continuous_front_profile"])
    back = curve(h, 1, c["continuous_back_profile"])
    axial = (
        c["height_m"] * (evaluate(h, data["segments"]) - front) * p["axial_smoothing"]
    )
    transverse = c["height_m"] * (back - front) * 0.72 * p["cheek_roundness"]
    transverse *= ease((h - 0.1) / 0.1) * ease((0.9 - h) / 0.1)
    # Cancel only the inherited abs(x) crease inside the medial fifth.
    # q - q*(1-q/r)^3 matches q, q' and q'' at r and has zero slope at 0.
    radius = 0.2
    center = 0.0
    if q < radius:
        center = -c["height_m"] * (back - front) * c["continuous_lateral_linear"]
        center *= min(1.0, 2 * p["axial_smoothing"])
        center *= q * (1 - q / radius) ** 3
        center *= ease((h - 0.1) / 0.1) * ease((0.9 - h) / 0.1)
    return (
        axial * response(q, data["transverse"]["axial"])
        + transverse * response(q, data["transverse"]["rounding"])
        + center
    )


def ocular_delta(x, y, data):
    plane = data["ocular_plane"]
    dx, dy = abs(x) - plane["center"][0], y - plane["center"][1]
    box = plane["domain_relative_m"]
    if (
        not box[0][0] - 1e-10 <= dx <= box[0][1] + 1e-10
        or not box[1][0] - 1e-10 <= dy <= box[1][1] + 1e-10
    ):
        raise ValueError("Ocular sample outside certified displacement-plane domain")
    a, b, d = plane["coefficients"]
    displacement = a * dx + b * dy + d
    # The authored slope is already in C; check its contribution without adding
    # it twice to geometry. Legacy standalone callers have no reviewed gate.
    combined = displacement + plane.get("authored_horizontal_slope_delta", 0.0) * dx
    if "REVIEWED_BOUND" in globals() and abs(combined) > REVIEWED_BOUND:
        raise ValueError("Actual ocular support exceeds reviewed displacement bound")
    return displacement


def blend_medial_normal(normal, smooth_normal, region, side, z, calibration, data):
    """Remove only the active upper-medial normal partition; zero is identity."""
    if region not in (1, 2) or side == 0:
        return normal
    amount = min(1.0, 2 * data["parameters"]["axial_smoothing"])
    if amount == 0:
        return normal
    height = (z + 0.021 - calibration["origin_y_m"]) / calibration["height_m"]
    weight = ease((height - 0.4) / 0.15) * amount
    return normal.lerp(smooth_normal, weight).normalized()
