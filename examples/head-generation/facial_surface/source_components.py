"""Small independent feature calibrations before procedural construction."""

from copy import deepcopy
import math

LIMITS = {
    "mouth_width_scale": (0.92, 1.08),
    "nose_bridge_scale": (1.0, 1.2),
    "ear_span_scale": (0.94, 1.0),
}


def apply_components(style, parameters=None):
    parameters = {} if parameters is None else parameters
    if not isinstance(parameters, dict) or set(parameters) - set(LIMITS):
        raise ValueError("Unknown component calibration")
    values = {}
    for key, (low, high) in LIMITS.items():
        value = parameters.get(key, 1.0)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid component calibration: " + key)
        values[key] = float(value)
    result = deepcopy(style)
    if all(value == 1 for value in values.values()):
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
    if values["nose_bridge_scale"] != 1:
        stations = c["nose_stations"]
        if [r[0] for r in stations] != [0.19, 0.22, 0.25, 0.286, 0.303, 0.34, 0.4]:
            raise ValueError("Nose station contract changed")
        before["nose_stations"] = deepcopy(stations)
        # Change the bridge shoulder only. Root, tip and nasal base stay fixed.
        stations[5][1] *= values["nose_bridge_scale"]
        after["nose_stations"] = deepcopy(stations)
    return result, {
        "parameters": values,
        "before": before,
        "after": after,
        "scope": "Pre-construction feature dimensions; unchanged connection algorithms",
        "bound_scope": "Surface correction bound excludes these baseline calibration changes; native shape and attachment checks required",
        "native_acceptance": "pending",
    }
