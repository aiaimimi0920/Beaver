"""Versioned face-width baseline templates; residual limits stay separate."""

import copy
import hashlib
import json
import math
from source_silhouette import HEIGHTS, bezier_segments


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    ).hexdigest()


def width_change_bound(original, modified, height):
    return height * max(
        abs(a - b)
        for p, q in zip(bezier_segments(original), bezier_segments(modified))
        for a, b in zip(p, q)
    )


def apply_family_templates(style, templates=None, residual_parameters=None):
    if templates is None or templates == []:
        return copy.deepcopy(style), None
    if not isinstance(templates, list) or len(templates) != 1:
        raise ValueError("Pilot supports exactly one face template")
    t = templates[0]
    required = {
        "id",
        "version",
        "slot",
        "reference_sources",
        "width_stations",
        "retained_width_sha256",
        "max_baseline_width_change_m",
        "attachment_interface",
        "residual_controls",
        "template_sha256",
    }
    if (
        not isinstance(t, dict)
        or set(t) != required
        or t["slot"] != "face"
        or type(t["version"]) is not int
        or (t["version"], t["attachment_interface"])
        not in (
            (1, "retained-ear-v1"),
            (2, "transported-ear-v2"),
            (3, "root-protected-ear-v3"),
        )
    ):
        raise ValueError("Unknown family template contract")
    if t["template_sha256"] != digest(
        {k: v for k, v in t.items() if k != "template_sha256"}
    ):
        raise ValueError("Family template content changed")
    if (
        not isinstance(t["id"], str)
        or not t["id"]
        or not isinstance(t["reference_sources"], list)
        or not t["reference_sources"]
        or any(
            not isinstance(v, str) or not v.startswith("https://")
            for v in t["reference_sources"]
        )
    ):
        raise ValueError("Template needs identity and reference provenance")
    if t["residual_controls"] != (
        {} if t["version"] == 3 else {"chin_width": [-0.04, 0.0]}
    ):
        raise ValueError("Unknown template residual compatibility contract")
    if residual_parameters is not None and not isinstance(residual_parameters, dict):
        raise ValueError("Expected residual parameter mapping")
    for key, value in (residual_parameters or {}).items():
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
        ):
            raise ValueError("Invalid template residual")
        if value == 0:
            continue
        if (
            key not in t["residual_controls"]
            or not t["residual_controls"][key][0]
            <= value
            <= t["residual_controls"][key][1]
        ):
            raise ValueError(
                "Residual is not supported by selected family template: " + key
            )
    c = style["calibration"]
    old = c["width_stations"]
    new = t["width_stations"]
    if (
        c["height_m"] != 0.236
        or c["origin_y_m"] != 1.544
        or digest(old) != t["retained_width_sha256"]
    ):
        raise ValueError("Retained family base contract changed")
    if (
        not isinstance(new, list)
        or len(new) != len(HEIGHTS)
        or any(not isinstance(row, list) or len(row) != 2 for row in new)
    ):
        raise ValueError("Template station shape mismatch")
    if any(
        isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v)
        for row in new
        for v in row
    ):
        raise ValueError("Template values must be finite numbers")
    if tuple(row[0] for row in new) != HEIGHTS or any(
        not 0.01 <= row[1] <= 0.45 for row in new
    ):
        raise ValueError("Template outside absolute width domain")
    base_curves = bezier_segments(old)
    new_curves = bezier_segments(new)
    for index, segment in enumerate(new_curves):
        direction = 1 if index < 9 else -1
        if min(segment) <= 0 or any(
            direction * (b - a) < -1e-12 for a, b in zip(segment, segment[1:])
        ):
            raise ValueError("Template violates its monotone segment contract")
    # h>=.45: same entire cubic, not merely same station values.
    if any(
        abs(a - b) > 1e-12
        for p, q in zip(
            base_curves[4 if t["version"] == 3 else 7 :],
            new_curves[4 if t["version"] == 3 else 7 :],
        )
        for a, b in zip(p, q)
    ):
        raise ValueError("Protected upper-face cubic changed")
    limit = t["max_baseline_width_change_m"]
    if (
        isinstance(limit, bool)
        or not isinstance(limit, (int, float))
        or not math.isfinite(limit)
        or not 0 < limit <= 0.012
    ):
        raise ValueError("Invalid explicit family baseline domain")
    bound = width_change_bound(old, new, c["height_m"])
    if bound > limit:
        raise ValueError("Template exceeds declared baseline width change")
    result = copy.deepcopy(style)
    result["calibration"]["width_stations"] = copy.deepcopy(new)
    return result, {
        "template": copy.deepcopy(t),
        "retained_width_stations": copy.deepcopy(old),
        "template_width_stations": copy.deepcopy(new),
        "template_half_width_change_bound_m": bound,
        "protected_height_range": [0.16 if t["version"] == 3 else 0.45, 1.0],
        "scope": "Authored width-template construction, not recovered dimensions or x-only deformation. Downstream retained ear constraints remain active and require native review.",
    }


def instrument_ear_constraints(source):
    replacements = {
        "    proposed=constrain_lower_envelope(before,result,changed)": "    proposed=constrain_lower_envelope(before,result,changed)\n    template_envelope_requested=max(((Vector(proposed[i])-Vector(safe[i])).length for i in changed),default=0)",
        "    # Preserve each small bridge coherently": "    template_opening_requested=max(((Vector(opening_proposed[i])-Vector(opening_safe[i])).length for i in changed),default=0)\n    # Preserve each small bridge coherently",
        "return {'opening_initial_invalid':": "return {'template_envelope_proposed_limited_max_m':template_envelope_requested,'template_opening_proposed_limited_max_m':template_opening_requested,'retained_shape_constraints_active':True,'constraint_diagnostics_scope':'already internally limited proposals, not raw unclipped demand','opening_initial_invalid':",
    }
    for old, new in replacements.items():
        if source.count(old) != 1:
            raise ValueError("Template attachment instrumentation contract changed")
        source = source.replace(old, new, 1)
    compile(source, "<template ear diagnostics>", "exec")
    return source


def transport_ear_constraints(source, retained_stations):
    """Transport attachment targets without changing correction or topology gates."""
    if "def family_width_delta(" in source:
        raise ValueError("Family attachment transport already applied")
    helper = (
        "\ndef family_width_delta(z):\n"
        "    from face import H,Y0,C,curve\n"
        "    v=(z+.021-Y0)/H\n"
        '    return H*(curve(v,1,C["width_stations"])-curve(v,1,'
        + repr(retained_stations)
        + "))\n"
    )
    replacements = {
        "                excess=q[0]*nx+q[1]*nz-support-.001": "                support += nx*family_width_delta(p[2])/height\n"
        "                excess=q[0]*nx+q[1]*nz-support-.001",
        "        target_x+=(1 if x>=0 else -1)*return_width": "        target_x+=(1 if x>=0 else -1)*family_width_delta(z)\n"
        "        target_x+=(1 if x>=0 else -1)*return_width",
    }
    for old, new in replacements.items():
        if source.count(old) != 1:
            raise ValueError("Family attachment transport source contract changed")
        source = source.replace(old, new, 1)
    source += helper
    compile(source, "<transported ear constraints>", "exec")
    return source
