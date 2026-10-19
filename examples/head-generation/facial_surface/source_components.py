"""Small independent feature calibrations before procedural construction."""

from copy import deepcopy
import math

LIMITS = {
    "mouth_width_scale": (0.92, 1.08),
    "nose_bridge_scale": (1.0, 1.2),
    "nose_bridge_length_scale": (1.0, 1.2),
    "nose_tip_projection_scale": (1.0, 1.08),
    "ear_span_scale": (0.94, 1.0),
    "ear_tip_length_scale": (1.0, 1.8),
    "ear_point_strength": (0.0, 1.0),
    "ear_tip_lift": (0.0, 0.04),
}


def apply_components(style, parameters=None):
    parameters = {} if parameters is None else parameters
    if not isinstance(parameters, dict) or set(parameters) - set(LIMITS):
        raise ValueError("Unknown component calibration")
    values = {}
    for key, (low, high) in LIMITS.items():
        default = 0.0 if key in ("ear_point_strength", "ear_tip_lift") else 1.0
        value = parameters.get(key, default)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid component calibration: " + key)
        values[key] = float(value)
    result = deepcopy(style)
    if all(
        value == (0.0 if key in ("ear_point_strength", "ear_tip_lift") else 1.0)
        for key, value in values.items()
    ):
        return result, None
    c = result["calibration"]
    before = {}
    after = {}
    for key in ("mouth_half_width", "upper_lip_lateral_width"):
        if values["mouth_width_scale"] != 1:
            before[key] = c[key]
            c[key] *= values["mouth_width_scale"]
            after[key] = c[key]
    if values["ear_span_scale"] != 1:
        before["ear_outward_span"] = c["ear_outward_span"]
        c["ear_outward_span"] *= values["ear_span_scale"]
        after["ear_outward_span"] = c["ear_outward_span"]
    if (
        values["ear_tip_length_scale"] != 1
        or values["ear_point_strength"] != 0
        or values["ear_tip_lift"] != 0
    ):
        for key in ("ear_tip_length_scale", "ear_point_strength", "ear_tip_lift"):
            before[key] = c.get(key)
            c[key] = values[key]
            after[key] = c[key]
    if any(
        values[k] != 1
        for k in (
            "nose_bridge_scale",
            "nose_bridge_length_scale",
            "nose_tip_projection_scale",
        )
    ):
        stations = c["nose_stations"]
        if [r[0] for r in stations] != [0.19, 0.22, 0.25, 0.286, 0.303, 0.34, 0.4]:
            raise ValueError("Nose station contract changed")
        before["nose_stations"] = deepcopy(stations)
        # Legacy shoulder-only choice remains exact. New family controls are explicit.
        stations[5][1] *= values["nose_bridge_scale"]
        # Lengthen the upper bridge about its existing lower landmark, not the whole face.
        anchor = stations[4][0]
        for row in stations[5:]:
            row[0] = anchor + (row[0] - anchor) * values["nose_bridge_length_scale"]
        # Local tip support; basal/root station projections and station widths stay fixed.
        # Interpolated widths and neighboring slopes may still change.
        for index, weight in ((2, 0.3), (3, 1.0), (4, 0.8)):
            stations[index][1] *= 1 + weight * (values["nose_tip_projection_scale"] - 1)
        if (
            not all(a[0] < b[0] for a, b in zip(stations, stations[1:]))
            or stations[-1][0] >= 0.43
        ):
            raise ValueError("Nose family exceeds inherited fairing height envelope")
        after["nose_stations"] = deepcopy(stations)
    return result, {
        "parameters": values,
        "before": before,
        "after": after,
        "scope": "Pre-construction feature dimensions; unchanged connection algorithms",
        "bound_scope": "Surface correction bound excludes these baseline calibration changes; native shape and attachment checks required",
        "native_acceptance": "pending",
    }
