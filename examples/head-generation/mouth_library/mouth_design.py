"""Bounded static mouth design; appearance is not a speech-state taxonomy."""

import copy
import math

LIMITS = {
    "half_width": (0.055, 0.085),
    "corner_lift": (-0.002, 0.002),
    "seam_bow": (0.0, 0.003),
    "lip_band_width_m": (0.00025, 0.0018),
    "lip_strength": (0.3, 0.8),
    "muzzle_projection": (0.0, 0.04),
    "muzzle_half_width": (0.10, 0.17),
}
KINDS = {"line", "lip-band", "muzzle"}


def validate(value):
    if value is None:
        return None
    if not isinstance(value, dict) or set(value) != set(LIMITS) | {"version", "kind"}:
        raise ValueError("Expected exact mouth design fields")
    if (
        type(value["version"]) is not int
        or value["version"] != 1
        or not isinstance(value["kind"], str)
        or value["kind"] not in KINDS
    ):
        raise ValueError("Unsupported mouth design version or kind")
    for name, (low, high) in LIMITS.items():
        v = value[name]
        if (
            isinstance(v, bool)
            or not isinstance(v, (int, float))
            or not math.isfinite(v)
            or not low <= v <= high
        ):
            raise ValueError("Invalid mouth parameter: " + name)
    if value["kind"] != "muzzle" and value["muzzle_projection"] != 0:
        raise ValueError("Only muzzle designs may project the mouth support")
    if value["kind"] == "muzzle" and value["muzzle_projection"] <= 0:
        raise ValueError("Muzzle design requires positive support projection")
    return copy.deepcopy(value)


def apply(style, design):
    d = validate(design)
    out = copy.deepcopy(style)
    if d is None:
        return out
    c = out["calibration"]
    if c.get("mouth_seam_bow_height", 0):
        raise ValueError("Cannot mix researched mouth and old contour template")
    c.update(
        mouth_design=d,
        mouth_half_width=d["half_width"],
        mouth_corner_lift=d["corner_lift"],
        mouth_seam_bow_height=d["seam_bow"],
        mouth_seam_shoulder_radius_ratio=0.7,
    )
    out["mouth_design"] = d
    out["lip_pigment_width_m"] = d["lip_band_width_m"]
    out["lip_pigment_strength"] = d["lip_strength"]
    return out
