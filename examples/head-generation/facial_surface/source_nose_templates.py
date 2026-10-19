"""Versioned rounded-nose construction with a matched native fairing interface."""

import ast
import copy
import math
from source_family_templates import digest
from source_mouth_templates import replace_once


def apply_nose_template(style, templates):
    if not templates:
        return copy.deepcopy(style), None
    if not isinstance(templates, list) or len(templates) != 1:
        raise ValueError("Expected one nose template")
    t = templates[0]
    keys = {
        "id",
        "version",
        "slot",
        "reference_sources",
        "retained_stations_sha256",
        "nose_stations",
        "cross_section",
        "fairing_interface",
        "template_sha256",
    }
    if (
        not isinstance(t, dict)
        or set(t) != keys
        or t["slot"] != "nose"
        or type(t["version"]) is not int
        or t["version"] != 1
        or t["cross_section"] != "rounded-quartic"
        or t["fairing_interface"] != "flat-apex-v1"
    ):
        raise ValueError("Unknown nose template contract")
    if t["template_sha256"] != digest(
        {k: v for k, v in t.items() if k != "template_sha256"}
    ):
        raise ValueError("Nose template changed")
    if (
        not isinstance(t["id"], str)
        or not t["id"]
        or not isinstance(t["reference_sources"], list)
        or not t["reference_sources"]
        or any(
            not isinstance(s, str) or not s.startswith("https://")
            for s in t["reference_sources"]
        )
    ):
        raise ValueError("Missing nose provenance")
    c = style["calibration"]
    old = c["nose_stations"]
    new = t["nose_stations"]
    if digest(old) != t["retained_stations_sha256"] or c["height_m"] != 0.236:
        raise ValueError("Nose base or residual contract changed")
    if (
        not isinstance(new, list)
        or len(new) != 7
        or any(not isinstance(r, list) or len(r) != 3 for r in new)
    ):
        raise ValueError("Nose station shape mismatch")
    if any(
        isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v)
        for r in new
        for v in r
    ):
        raise ValueError("Nose station values must be finite")
    if any(a[0] != b[0] or a[2] != b[2] for a, b in zip(old, new)) or any(
        new[i] != old[i] for i in (0, 1, 6)
    ):
        raise ValueError("Nose attachment landmarks changed")
    if any(not 0.7 * a[1] <= b[1] <= a[1] for a, b in zip(old, new)):
        raise ValueError("Nose projection outside explicit construction domain")
    if not all(new[i][1] <= new[i + 1][1] for i in range(3)) or not all(
        new[i][1] >= new[i + 1][1] for i in range(3, 6)
    ):
        raise ValueError("Nose projection must have one apex")
    out = copy.deepcopy(style)
    out["calibration"]["nose_stations"] = copy.deepcopy(new)
    out["calibration"]["nose_template_mode"] = "rounded-quartic"
    return out, {
        "template": copy.deepcopy(t),
        "retained_stations": copy.deepcopy(old),
        "projection_support_h": [0.19, 0.4],
        "endpoint_projection_slope": 0,
        "cross_section_center_and_outer_slope": 0,
        "scope": "Authored short-rounded baseline and matched flat-apex fairing; existing 3mm refinement and 0.8mm fairing caps retained, not a global displacement or final appearance certificate",
    }


def adapt_nose_face(source, enabled=False):
    if not enabled:
        return source
    node = next(
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == "curve"
    )
    raw_curve = ast.get_source_segment(source, node)
    if any(
        raw_curve.count(anchor) != 1
        for anchor in ("def curve(", "[slopes[0]]", "[slopes[-1]]")
    ):
        raise ValueError("Nose projection curve source contract changed")
    helper = (
        raw_curve.replace("def curve(", "def nasal_projection_curve(", 1)
        .replace("[slopes[0]]", "[0.0]", 1)
        .replace("[slopes[-1]]", "[0.0]", 1)
    )
    source = replace_once(
        source,
        "def nose_profile_factor(t, v):",
        helper + "\n\ndef nose_profile_factor(t, v):",
    )
    node = next(
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == "nose_profile_factor"
    )
    source = replace_once(
        source,
        ast.get_source_segment(source, node),
        "def nose_profile_factor(t, v):\n    t=max(0,min(1,t))\n    return (1-t*t)**2",
    )
    return replace_once(
        source, "h = curve(v, 1, pts) * H", "h = nasal_projection_curve(v, 1, pts) * H"
    )


def adapt_nose_companion(name, source, enabled=False):
    if not enabled:
        return source
    if name == "nose_refine.py":
        source = replace_once(
            source,
            "apex = .0022*bump((height-.298)/apex_radius)",
            "apex = 0.0*bump((height-.298)/apex_radius)",
        )
        return replace_once(
            source,
            "'apex_projection_m':.0022",
            "'apex_projection_m':0.0,'template_interface':'flat-apex-v1'",
        )
    if name == "nose_fair.py":
        source = replace_once(
            source,
            "lower=fit_arc(points[:peak+1],0.0,.2)",
            "lower=fit_arc(points[:peak+1],0.0,0.0)",
        )
        source = replace_once(
            source,
            "upper=fit_arc(points[peak:],-1.35,-.12)",
            "upper=fit_arc(points[peak:],0.0,-.12)",
        )
        return replace_once(
            source,
            "'kind':'two quintic arcs with fixed endpoints and apex'",
            "'kind':'two quintic arcs with fixed endpoints and zero apex tangent targets','template_interface':'flat-apex-v1','lower_applied_strength':.35,'upper_applied_strength':1.0",
        )
    raise ValueError("Unknown nose companion")
