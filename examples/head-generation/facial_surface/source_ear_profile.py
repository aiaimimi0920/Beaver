"""Opt-in pointed distal ear construction; preserve the existing root boundary chain."""

import math


def adapt_ear_profile(source, calibration):
    controls = {
        "ear_tip_length_scale": (1.0, 1.0, 1.8),
        "ear_point_strength": (0.0, 0.0, 1.0),
        "ear_tip_lift": (0.0, 0.0, 0.04),
    }
    values = {}
    for key, (default, low, high) in controls.items():
        value = calibration.get(key, default)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid ear profile: " + key)
        values[key] = value
    if all(values[k] == v[0] for k, v in controls.items()):
        return source
    old = '    outward = envelope * config["ear_outward_span"] * curve(u, 1, config["ear_outward_profile"])'
    new = """    outward_envelope = (1-config.get("ear_point_strength", 0)) * envelope + config.get("ear_point_strength", 0) * pointed_ear_envelope(t)
    original_outward = envelope * config["ear_outward_span"] * curve(u, 1, config["ear_outward_profile"])
    target_outward = outward_envelope * config["ear_outward_span"] * config.get("ear_tip_length_scale", 1) * curve(u, 1, config["ear_outward_profile"])
    outward = original_outward + ear_distal_weight(t,u) * (target_outward-original_outward)"""
    if source.count(old) != 1:
        raise ValueError("Ear outward profile contract changed")
    source = source.replace(old, new, 1)
    old = "                    lift = (ear_arc_height(t, low, high) - root.z) * sqrt(max(0, sin(pi * u / 2)))"
    new = (
        old
        + '\n                    lift += H * C.get("ear_tip_lift", 0) * pointed_ear_envelope(t) * u * u * ear_distal_weight(t,u)'
    )
    if source.count(old) != 1:
        raise ValueError("Ear distal lift contract changed")
    source = source.replace(old, new, 1)
    source += """

def ear_distal_weight(t,u):
    # Preserve the proximal columns and lower attachment neighborhood exactly.
    a = max(0.0,min(1.0,(u-.35)/.65))
    b = max(0.0,min(1.0,(t-.25)/.30))
    return a*a*(3-2*a)*b*b*(3-2*b)


def pointed_ear_envelope(t):
    t = max(0.0, min(1.0, t))
    peak = 0.70
    return t/peak if t <= peak else (1-t)/(1-peak)
"""
    compile(source, "<distal ear profile>", "exec")
    return source
