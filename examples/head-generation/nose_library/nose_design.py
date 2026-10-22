"""Bounded authored nose controls; ranges are not measurements of the corpus."""

import copy
import hashlib
import json
import math

KINDS = {"fused", "pad-muzzle", "faceted-candidate"}
LIMITS = {
    "root_height": (0.36, 0.43),
    "tip_height": (0.265, 0.31),
    "projection": (0.055, 0.115),
    "half_width": (0.04, 0.075),
    "roundness": (0.0, 1.0),
    "muzzle_width": (0.075, 0.13),
    "muzzle_projection": (0.012, 0.04),
    "pad_width": (0.025, 0.05),
    "pad_height": (0.016, 0.032),
    "pad_projection": (0.008, 0.022),
}
COMMON = {"root_height", "tip_height", "projection", "half_width", "roundness"}
SPECIAL = {
    "muzzle_width",
    "muzzle_projection",
    "pad_width",
    "pad_height",
    "pad_projection",
}


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    ).hexdigest()


def validate(value):
    if value is None:
        return None
    if not isinstance(value, dict) or value.get("kind") not in KINDS:
        raise ValueError("Unknown researched nose kind")
    owned = COMMON | (SPECIAL if value["kind"] == "pad-muzzle" else set())
    if (
        set(value) != {"version", "kind"} | owned
        or type(value["version"]) is not int
        or value["version"] != 1
    ):
        raise ValueError("Nose design schema mismatch")
    out = copy.deepcopy(value)
    for key in owned:
        v = value[key]
        if (
            type(v) not in (int, float)
            or not math.isfinite(v)
            or not LIMITS[key][0] <= v <= LIMITS[key][1]
        ):
            raise ValueError("Nose control outside authored domain: " + key)
        out[key] = float(v)
    if out["root_height"] - out["tip_height"] < 0.065:
        raise ValueError("Nose root and tip must retain positive bridge span")
    return out


def stations(design):
    d = validate(design)
    tip, root, p, w = (
        d[k] for k in ("tip_height", "root_height", "projection", "half_width")
    )
    lower = 0.19
    # Shoulder fullness changes continuously while endpoints remain tangent-flat.
    shoulder = 0.38 + 0.25 * d["roundness"]
    if d["kind"] == "faceted-candidate":
        p *= 0.12  # Only a low support bed; the visible structure is a planar module.
    return [
        [lower, 0.0, w * 0.7],
        [lower + (tip - lower) * 0.28, p * 0.12, w * 0.85],
        [lower + (tip - lower) * 0.67, p * shoulder, w],
        [tip, p, w],
        [tip + (root - tip) * 0.18, p * (0.78 + 0.16 * d["roundness"]), w * 0.96],
        [tip + (root - tip) * 0.65, p * 0.28, w * 0.7],
        [root, 0.0, w * 0.6],
    ]
