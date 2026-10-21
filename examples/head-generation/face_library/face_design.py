"""Bounded authored lower-face profiles; not measurements or a skull taxonomy."""

import copy
import hashlib
import json
import math
from pathlib import Path

HEIGHTS = (0, .03, .07, .12, .16, .20, .30, .45, .60, .75, .85, 1)
RETAINED = (.025, .088, .165, .244, .276, .29, .325, .37, .402, .417, .407, .4)
ANCHORS = {
    "compact-rounded": (.032, .115, .202, .263, .284, .295),
    "soft-tapered": (.028, .094, .178, .246, .276, .29),
    "straight-tapered": (.025, .075, .145, .220, .260, .286),
    "blunt-chin": (.065, .119, .186, .248, .278, .29),
}
RETAINED_SHAPE_HASHES = {
    "continuous_front_profile": "ca1fcdfb93be6131c9ff24a28e0c007c69e8aa188b908e5668bb56243d59288b",
    "cross_sections": "bf84aed0040bdb84149e0e0caaa8311db982972b91a415186a0e90829b9e0421",
}
LIMITS = {
    "jaw_fullness": (-.004, .004),
    "chin_roundness": (0., 1.),
    "chin_projection": (-.006, .006),
}


def digest(value):
    serialized = json.dumps(
        value, sort_keys=True, separators=(",", ":"), allow_nan=False
    )
    return hashlib.sha256(serialized.encode()).hexdigest()


def provenance():
    return {"face_design.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}


def validate(value):
    if value is None:
        return None
    if not isinstance(value, dict) or set(value) != {"version", "kind"} | set(LIMITS):
        raise ValueError("Face design schema mismatch")
    if (
        type(value["version"]) is not int
        or value["version"] != 1
        or value["kind"] not in ANCHORS
    ):
        raise ValueError("Unknown face design")
    out = copy.deepcopy(value)
    for key, (lo, hi) in LIMITS.items():
        v = value[key]
        if type(v) not in (int, float) or not math.isfinite(v) or not lo <= v <= hi:
            raise ValueError("Face control outside authored domain: " + key)
        out[key] = float(v)
    return out


def bezier(points):
    slopes = [(b[1] - a[1]) / (b[0] - a[0]) for a, b in zip(points, points[1:])]
    tangents = (
        [slopes[0]]
        + [0 if a * b <= 0 else 2 * a * b / (a + b)
           for a, b in zip(slopes, slopes[1:])]
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


def difference_bound(before, after, height):
    return height * max(
        abs(a - b)
        for old, new in zip(bezier(before), bezier(after))
        for a, b in zip(old, new)
    )


def stations(design):
    d = validate(design)
    widths = list(ANCHORS[d["kind"]]) + list(RETAINED[6:])
    fullness = (0, .15, .65, 1., .7, .2)
    # End curvature and overall taper are separate authored controls.
    for i, weight in enumerate(fullness):
        widths[i] += d["jaw_fullness"] * weight
    widths[0] += .006 * (d["chin_roundness"] - .5)
    widths[1] += .003 * (d["chin_roundness"] - .5)
    points = [[h,w] for h,w in zip(HEIGHTS,widths)]
    curves = bezier(points)
    for i, segment in enumerate(curves):
        sign = 1 if i < 9 else -1
        if min(segment) <= 0 or any(sign*(b-a) < -1e-12 for a,b in zip(segment,segment[1:])):
            raise ValueError("Face width profile must remain positive and monotone")
    return points


def projection_weight(height):
    t = max(0., min(1., height / .18))
    return (1-t*t)**2


def researched_underside_lift(config, lift):
    t = max(0.0, min(1.0, lift / config["chin_underside_stations"][-1][1]))
    return config["chin_return_lift"] + (
        config["researched_chin_underside_lift"] - config["chin_return_lift"]
    ) * t * t * (3 - 2 * t)


def apply_design(style, design):
    d = validate(design)
    out = copy.deepcopy(style)
    if d is None:
        return out, None
    c = out["calibration"]
    old = c["width_stations"]
    if (
        c["height_m"] != .236
        or c["origin_y_m"] != 1.544
        or old != [list(x) for x in zip(HEIGHTS, RETAINED)]
        or c.get("chin_return_lift") != .008
        or "researched_chin_underside_lift" in c
    ):
        raise ValueError("Face design requires the retained calibration")
    if any(digest(c.get(key)) != expected for key, expected in RETAINED_SHAPE_HASHES.items()):
        raise ValueError("Retained face depth calibration changed")
    new = stations(d)
    old_curves, new_curves = bezier(old), bezier(new)
    bound = difference_bound(old, new, c["height_m"])
    if bound > .012:
        raise ValueError("Face template exceeds existing 12 mm baseline domain")
    if old_curves[7:] != new_curves[7:]:
        raise ValueError("Protected upper-face width interpolant changed")
    c["width_stations"] = new
    retained_lift = c["chin_return_lift"]
    # Round the actual underside, rather than only widening its frontal outline.
    lift_delta = .016 * d["chin_roundness"] if d["kind"] == "compact-rounded" else 0.0
    c["researched_chin_underside_lift"] = retained_lift + lift_delta
    profile_before = copy.deepcopy(c["continuous_front_profile"])
    sections_before = copy.deepcopy(c["cross_sections"])
    c["continuous_front_profile"] = [
        [h, z + d["chin_projection"] * projection_weight(h)]
        for h, z in profile_before
    ]
    c["cross_sections"] = [
        [h, [[x, z + d["chin_projection"] * projection_weight(h)]
             for x, z in points]]
        for h, points in sections_before
    ]
    profile_bound = difference_bound(
        profile_before, c["continuous_front_profile"], c["height_m"]
    )
    # Convex-hull bound of the actual profile interpolant, not only its knots.
    if profile_bound > .002:
        raise ValueError("Chin profile exceeds 2 mm depth domain")
    out["face_design"] = d
    record = {
        "parameters": d,
        "source_sha256": provenance(),
        "retained_width_stations": old,
        "retained_chin_return_lift": retained_lift,
        "chin_underside_lift": c["researched_chin_underside_lift"],
        "chin_return_vertical_displacement_bound_m": c["height_m"] * lift_delta,
        "width_stations": copy.deepcopy(new),
        "retained_front_profile": profile_before,
        "front_profile": copy.deepcopy(c["continuous_front_profile"]),
        "retained_cross_sections": sections_before,
        "cross_sections": copy.deepcopy(c["cross_sections"]),
        "continuous_half_width_change_bound_m": bound,
        "front_profile_interpolant_change_bound_m": profile_bound,
        "protected_width_height_range": [.45,1.],
        "scope": (
            "Authored lower-face calibration, not measured reference dimensions, "
            "complete skull geometry, global collision proof or aesthetic acceptance. "
            "Vertical face length is not an independent control."
        ),
    }
    return out, record
