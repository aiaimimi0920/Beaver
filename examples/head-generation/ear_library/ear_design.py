"""Authored lateral-ear seeds, not measured anatomy or an exhaustive taxonomy."""

import copy
import hashlib
import json
import math
from pathlib import Path

LIMITS = {
    "root_height_offset": (-.01, .01),
    "root_span_scale": (.9, 1.1),
    "outward_scale": (.85, 1.15),
    "posterior_scale": (.8, 1.2),
    "tip_height": (-.02, .02),
    "tip_roundness": (0., 1.),
    "lobe_fullness": (-.1, .1),
    "fold_strength": (0., 1.),
}
# low/high attachment, lateral/posterior span, upper fullness, lower taper,
# upper trim, arc bias, point strength/peak, return-lip width.
SEEDS = {
    "standard-oval": (.26, .53, .095, .112, .32, .20, .14, .065, 0., .70, .14),
    "compact-round": (.26, .48, .108, .10, .12, .10, .10, .04, 0., .65, .12),
    "slender-oval": (.26, .53, .070, .115, .35, .26, .12, .04, 0., .70, .10),
    "short-point": (.26, .53, .118, .105, .25, .20, .10, .04, .55, .72, .10),
    "long-leaf": (.34, .50, .245, .035, .15, .25, .08, .015, .85, .60, .065),
    "side-droop": (.29, .53, .17, .105, -.22, .05, .08, -.07, .45, .32, .055),
}
CALIBRATION_KEYS = (
    "ear_root_min_height", "ear_root_max_height", "ear_outward_span",
    "ear_posterior_span", "ear_upper_fullness", "ear_lower_taper",
    "ear_upper_trim", "ear_upper_arc_bias", "researched_ear_point_strength",
    "researched_ear_point_peak", "ear_return_lip_width",
)


def digest(value):
    return hashlib.sha256(json.dumps(
        value, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()).hexdigest()


def provenance():
    root = Path(__file__).resolve().parent
    return {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
            for name in ("ear_design.py", "ear_profile.py")}


def validate(value):
    if value is None:
        return None
    if not isinstance(value, dict) or set(value) != {"version", "kind"} | set(LIMITS):
        raise ValueError("Ear design schema mismatch")
    if type(value["version"]) is not int or value["version"] != 1:
        raise ValueError("Unknown ear design version")
    if not isinstance(value["kind"], str) or value["kind"] not in SEEDS:
        raise ValueError("Unknown lateral ear seed")
    result = copy.deepcopy(value)
    for key, (low, high) in LIMITS.items():
        v = value[key]
        if type(v) not in (int, float) or not math.isfinite(v) or not low <= v <= high:
            raise ValueError("Ear control outside authored domain: " + key)
        result[key] = float(v)
    return result


def calibration(design):
    d = validate(design)
    result = dict(zip(CALIBRATION_KEYS, SEEDS[d["kind"]]))
    low, high = result["ear_root_min_height"], result["ear_root_max_height"]
    center = (low + high) / 2 + d["root_height_offset"]
    half_span = (high - low) * d["root_span_scale"] / 2
    result["ear_root_min_height"] = center - half_span
    result["ear_root_max_height"] = center + half_span
    result["ear_outward_span"] *= d["outward_scale"]
    result["ear_posterior_span"] *= d["posterior_scale"]
    result["ear_lower_taper"] -= d["lobe_fullness"]
    result["researched_ear_point_strength"] *= 1 - d["tip_roundness"]
    result["researched_ear_tip_height"] = d["tip_height"]
    result["researched_ear_kind"] = d["kind"]
    result["researched_ear_design_sha256"] = digest(d)
    return result


def apply_design(style, design):
    if design is None:
        return copy.deepcopy(style), None
    d = validate(design)
    result = copy.deepcopy(style)
    c = result["calibration"]
    c.update(calibration(d))
    # The researched profile owns the distal shape; never apply legacy stretch twice.
    c.update(ear_tip_length_scale=1., ear_point_strength=0., ear_tip_lift=0.)
    folds = ("ear_inner_fold_strength", "ear_inner_fork_strength", "ear_lobe_curl_strength")
    retained_folds = {key: result[key] for key in folds}
    for key in folds:
        result[key] *= d["fold_strength"]
    if d["kind"] == "side-droop":
        result["ear_inner_fork_strength"] = 0.
        result["ear_lobe_curl_strength"] = 0.
    result["ear_design"] = copy.deepcopy(d)
    return result, {
        "parameters": d,
        "source_sha256": provenance(),
        "calibration": calibration(d),
        "retained_fold_strengths": retained_folds,
        "scope": "Static authored lateral pinna; no crown attachment, metric anatomy or ear-motion proof",
    }
