"""Opt-in width-station experiments; no vertex scaling or baseline mutation."""

from copy import deepcopy
import math

HEIGHTS = (0, 0.03, 0.07, 0.12, 0.16, 0.20, 0.30, 0.45, 0.60, 0.75, 0.85, 1)
AXES = {
    "face_width": (0.02, (0, 0, 0, 0, 0, 0.25, 0.75, 1, 1, 0.5, 0, 0)),
    "jaw_width": (0.04, (0, 0, 0.25, 0.75, 1, 1, 0.25, 0, 0, 0, 0, 0)),
    "chin_width": (0.08, (1, 1, 0.3, 0, 0, 0, 0, 0, 0, 0, 0, 0)),
}
MAX_HALF_WIDTH_CHANGE_M = 0.002


def bezier_segments(points):
    """Bezier form of the exact inherited harmonic-slope cubic interpolant."""
    slopes = [(b[1] - a[1]) / (b[0] - a[0]) for a, b in zip(points, points[1:])]
    tangents = (
        [slopes[0]]
        + [0 if a * b <= 0 else 2 * a * b / (a + b) for a, b in zip(slopes, slopes[1:])]
        + [slopes[-1]]
    )
    return [
        (
            a[1],
            a[1] + (b[0] - a[0]) * tangents[i] / 3,
            b[1] - (b[0] - a[0]) * tangents[i + 1] / 3,
            b[1],
        )
        for i, (a, b) in enumerate(zip(points, points[1:]))
    ]


def validate(parameters):
    if parameters is None:
        parameters = {}
    if not isinstance(parameters, dict) or set(parameters) - (
        set(AXES) | {"chin_softness"}
    ):
        raise ValueError("Unknown silhouette controls")
    result = {}
    for name, (limit, _) in AXES.items():
        value = parameters.get(name, 0)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or abs(value) > limit
        ):
            raise ValueError("Invalid silhouette control: " + name)
        result[name] = value
    softness = parameters.get("chin_softness", 0.0)
    if (
        isinstance(softness, bool)
        or not isinstance(softness, (int, float))
        or not math.isfinite(softness)
        or not 0 <= softness <= 1
    ):
        raise ValueError("Invalid silhouette control: chin_softness")
    result["chin_softness"] = softness
    return result


def apply_silhouette(style, parameters):
    controls = validate(parameters)
    result = deepcopy(style)
    if not any(controls.values()):
        return result, None
    calibration = result["calibration"]
    chin_before = calibration["chin_return_lift"]
    if controls["chin_softness"]:
        if chin_before != 0.008 or calibration["origin_y_m"] != 1.544:
            raise ValueError("Chin curvature calibration contract changed")
        calibration["chin_return_lift"] = (
            chin_before - 0.004 * controls["chin_softness"]
        )
    old = calibration["width_stations"]
    if tuple(p[0] for p in old) != HEIGHTS:
        raise ValueError("Silhouette station contract changed")
    if any(not math.isfinite(w) or w <= 0 for _, w in old):
        raise ValueError("Invalid original width stations")
    height = calibration["height_m"]
    if not math.isfinite(height) or height <= 0:
        raise ValueError("Invalid face height")
    new = [
        [h, w * (1 + sum(controls[k] * AXES[k][1][i] for k in AXES))]
        for i, (h, w) in enumerate(old)
    ]
    original, modified = bezier_segments(old), bezier_segments(new)
    # Convex-hull difference bound covers every point, not only width stations.
    bound = (
        height
        * max(abs(b - a) for p, q in zip(original, modified) for a, b in zip(p, q))
        + 1e-12
    )
    if bound > MAX_HALF_WIDTH_CHANGE_M:
        raise ValueError("Combined silhouette exceeds 2 mm half-width budget")
    for p, q in zip(original, modified):
        direction = 1 if p[-1] > p[0] else -1
        if min(q) <= 0 or any(direction * (b - a) < -1e-12 for a, b in zip(q, q[1:])):
            raise ValueError("Silhouette positivity or monotone segment failed")
    calibration["width_stations"] = new
    return result, {
        "parameters": controls,
        "chin_return_lift_before": chin_before,
        "chin_return_lift_after": calibration["chin_return_lift"],
        "chin_direct_vertical_change_bound_m": height
        * 0.004
        * controls["chin_softness"],
        "chin_bound_scope": "Before downstream fairing, y>=1.544 and |q|<=1; not final 3D displacement. Other supports have exponentially small nonzero influence.",
        "original_width_stations": old,
        "final_width_stations": new,
        "continuous_half_width_change_bound_m": bound,
        "half_width_budget_m": MAX_HALF_WIDTH_CHANGE_M,
        "scope": "Width interpolant and opt-in chin vertical curvature; native attachment, mouth and ear checks required",
    }


def adapt_ear_envelope(source, original_stations):
    """Carry the opt-in width delta through the inherited ear-under envelope."""
    old_import = "from face import H,Y0,Z0"
    old_target = "delta=max(-.003,min(.003,w*height-current))"
    if source.count(old_import) != 1 or source.count(old_target) != 1:
        raise ValueError("Inherited ear envelope source contract changed")
    source = source.replace(old_import, "from face import H,Y0,Z0,C,curve", 1)
    replacement = (
        "v=(p.z+.021-Y0)/H\n"
        '        silhouette_delta=H*(curve(v,1,C["width_stations"])-curve(v,1,'
        + repr(original_stations)
        + "))\n"
        "        delta=max(-.003,min(.003,w*height+silhouette_delta-current))"
    )
    return source.replace(old_target, replacement, 1)
