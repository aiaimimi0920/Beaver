"""Explicit source adaptation and frozen provenance for researched nose designs."""

import ast
import copy
import hashlib
from pathlib import Path
from nose_design import validate, stations, digest

HERE = Path(__file__).resolve().parent
SOURCES = ("nose_design.py", "nose_field.py", "nose_mesh.py", "nose_source.py")


def retained_sources():
    return {name: (HERE / name).read_text(encoding="utf-8") for name in SOURCES}


def provenance():
    return {
        name: hashlib.sha256(source.encode()).hexdigest()
        for name, source in retained_sources().items()
    }


def apply_design(style, design):
    d = validate(design)
    if d is None:
        return copy.deepcopy(style), None
    out = copy.deepcopy(style)
    c = out["calibration"]
    if c.get("nose_template_mode"):
        raise ValueError("Researched design cannot mix with a legacy nose template")
    before = copy.deepcopy(c["nose_stations"])
    c["nose_stations"] = stations(d)
    c["nose_template_mode"] = "rounded-quartic"
    c["nose_design"] = d
    out["nose_design"] = d
    return out, {
        "parameters": d,
        "source_sha256": provenance(),
        "retained_stations": before,
        "stations": c["nose_stations"],
        "authored_relief_upper_bound_m": c["height_m"]
        * (d["projection"] + d.get("muzzle_projection", 0)),
        "scope": "Authored analytic relief envelope, not displacement from baseline or global collision proof",
    }


def replace_function(source, name, replacement):
    nodes = [
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == name
    ]
    if len(nodes) != 1:
        raise ValueError("Missing unique nose source function: " + name)
    n = nodes[0]
    lines = source.splitlines(keepends=True)
    return (
        "".join(lines[: n.lineno - 1])
        + replacement
        + "\n"
        + "".join(lines[n.end_lineno :])
    )


def adapt_face(source, design):
    if design is None:
        return source
    from source_nose_templates import adapt_nose_face

    source = adapt_nose_face(source, True)
    field = (HERE / "nose_field.py").read_text()
    source = replace_function(
        source,
        "nose_profile_factor",
        'def nose_profile_factor(t, v):\n    q=(1-max(0,min(1,t))**2)**2\n    r=C["nose_design"]["roundness"]\n    return (1-r)*q*q+r*q\n',
    )
    source = replace_function(
        source,
        "nose",
        'def nose(x, y):\n    v=(y-Y0)/H\n    pts=C["nose_stations"]\n    value=researched_muzzle(x,y)\n    if pts[0][0]<v<pts[-1][0]:\n        height=nasal_projection_curve(v,1,pts)*H\n        width=curve(v,2,pts)*H\n        value+=height*nose_profile_factor(abs(x)/width,v)\n    return value\n',
    )
    return field + "\n" + source


def companion(name, source, design):
    if design is None:
        return source
    # These legacy shape modifications are not valid for the new analytic field.
    # Replace rather than broaden their historical displacement assertions.
    fn = "refine_nose" if name == "nose_refine.py" else "fair_nasal_arcs"
    return f'''"""New-design analytic geometry: no legacy post-shaping is applied."""

def bump(t):
    return max(0.0,1.0-t*t)**3 if abs(t)<1 else 0.0

def {fn}(data):
    import math
    if any(not math.isfinite(v) for p in data.vertices for v in p.co):
        raise ValueError("Nonfinite researched nose mesh")
    return {{"kind":"analytic researched nose, legacy post-shaping not applicable", "selected_vertices":0, "max_delta_m":0.0, "outside_nose_unchanged":True, "scope":"No post-stage mutation; analytic generation is validated separately"}}
'''


def adapt_pipeline(source, design):
    if design is None:
        return source
    anchor = "PARTS[:] = stable_part_order(PARTS)"
    if source.count(anchor) != 1:
        raise ValueError("Stable semantic part ordering changed")
    insertion = '\nfrom researched_nose_runtime import build_attachment\n_nose_prior_ordinals = dict(semantic_part_ordinals)\nbuild_attachment(STYLE["nose_design"], shell)\nfor _part in PARTS:\n    if _part.name not in semantic_part_ordinals:\n        semantic_part_ordinals[_part.name] = max(semantic_part_ordinals.values(), default=9)+1\nassert all(semantic_part_ordinals[k]==v for k,v in _nose_prior_ordinals.items()), "Original semantic regions changed"\n'
    source = source.replace(anchor, anchor + insertion, 1)
    check_anchor = '        report["ear_shared_seams"] = verify_ear_seams(data, report["ear_root_fit"])'
    if source.count(check_anchor) != 1:
        raise ValueError("Final skin attachment validation hook changed")
    return source.replace(
        check_anchor,
        check_anchor
        + '\n        from researched_nose_runtime import validate_final_attachments\n        report["nose_final_attachment_checks"] = validate_final_attachments(data)',
        1,
    )
