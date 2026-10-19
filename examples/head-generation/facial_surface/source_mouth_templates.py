"""Opt-in shared mouth contour construction; retained source remains unchanged."""

import copy
import math
from source_family_templates import digest


def split_templates(templates):
    if templates is None:
        return [], []
    if not isinstance(templates, list):
        raise ValueError("Expected a template list")
    slots = [t.get("slot") if isinstance(t, dict) else None for t in templates]
    if any(s not in ("face", "mouth") for s in slots) or len(slots) != len(set(slots)):
        raise ValueError("Duplicate or unsupported component template")
    return [t for t in templates if t["slot"] == "face"], [
        t for t in templates if t["slot"] == "mouth"
    ]


def apply_mouth_template(style, templates):
    if not templates:
        return copy.deepcopy(style), None
    if len(templates) != 1:
        raise ValueError("Expected one mouth template")
    t = templates[0]
    required = {
        "id",
        "slot",
        "version",
        "reference_sources",
        "seam_bow_height",
        "shoulder_radius_ratio",
        "template_sha256",
    }
    if (
        set(t) != required
        or t["slot"] != "mouth"
        or type(t["version"]) is not int
        or t["version"] != 1
    ):
        raise ValueError("Unknown mouth template contract")
    if t["template_sha256"] != digest(
        {k: v for k, v in t.items() if k != "template_sha256"}
    ):
        raise ValueError("Mouth template changed")
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
        raise ValueError("Mouth template needs identity and sources")
    a = t["seam_bow_height"]
    radius = t["shoulder_radius_ratio"]
    if (
        isinstance(a, bool)
        or not isinstance(a, (int, float))
        or not math.isfinite(a)
        or not 0 < a <= 0.004
        or radius != 0.7
        or isinstance(radius, bool)
    ):
        raise ValueError("Unsupported mouth contour domain")
    c = style["calibration"]
    if (
        c["mouth_rest_half_gap"] <= 0
        or c["mouth_half_width"] <= 0
        or c["height_m"] != 0.236
    ):
        raise ValueError("Unsupported mouth base")
    result = copy.deepcopy(style)
    result["calibration"]["mouth_seam_bow_height"] = a
    result["calibration"]["mouth_seam_shoulder_radius_ratio"] = radius
    roots = [math.sqrt((6 + s * math.sqrt(21)) / 15) for s in (-1, 1)]
    derivative = max(abs(13.5 * x * (1 - x * x) * (1 - 3 * x * x)) for x in roots)
    return result, {
        "template": copy.deepcopy(t),
        "seam_height_change_bound_m": c["height_m"] * a,
        "seam_slope_change_bound": a * derivative / (radius * c["mouth_half_width"]),
        "scope": "Authored neutral contour and corresponding lip field shift; not final mesh displacement or collision proof",
    }


HELPERS = """
def mouth_profile_shift(x):
    s=x/(H*C["mouth_half_width"]*C["mouth_seam_shoulder_radius_ratio"])
    if abs(s)>=1: return 0.0
    return H*C["mouth_seam_bow_height"]*(27/4)*s*s*(1-s*s)**2


def mouth_rest_seam_y(x):
    t=max(-1,min(1,x/(H*C["mouth_half_width"])))
    return Y0+H*C["mouth_height"]+H*C["mouth_corner_lift"]*t*t+mouth_profile_shift(x)

"""


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Mouth template source contract changed: " + old[:60])
    result = source.replace(old, new, 1)
    compile(result, "<mouth template>", "exec")
    return result


def adapt_mouth_face(source, enabled=False):
    if not enabled:
        return source
    return replace_once(
        source,
        "def lip_detail_depth(x, y):\n    z = 0.0",
        HELPERS
        + "def lip_detail_depth(x, y):\n    y -= mouth_profile_shift(x)\n    z = 0.0",
    )


def adapt_mouth_companions(name, source, enabled=False):
    if not enabled:
        return source
    if name == "mouth.py":
        return replace_once(
            source,
            'return MOUTH_Y + H * C["mouth_corner_lift"] * t * t',
            "return mouth_rest_seam_y(x)",
        )
    if name == "textures.py":
        return replace_once(
            source,
            'seam = Y0 + H * (C["mouth_height"] + C["mouth_corner_lift"] * t * t)',
            "seam = mouth_rest_seam_y(x)",
        )
    if name == "crease_rules.py":
        source = replace_once(
            source,
            "def assign_creases(shell, front_count, height, origin, width_fn, config):",
            "def assign_creases(shell, front_count, height, origin, width_fn, config):\n    from face import mouth_rest_seam_y",
        )
        return replace_once(
            source,
            'abs(y - origin - height * config["mouth_height"]) < 0.002',
            "abs(y - mouth_rest_seam_y(x)) < 0.002",
        )
    raise ValueError("Unknown mouth companion")
