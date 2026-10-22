"""Explicit researched mouth adaptation, provenance and export extension."""

import hashlib
from pathlib import Path
from mouth_design import validate, apply

HERE = Path(__file__).resolve().parent
SOURCES = (
    "mouth_design.py",
    "mouth_field.py",
    "mouth_motion.py",
    "mouth_morphs.py",
    "mouth_source.py",
)


def retained_sources():
    return {name: (HERE / name).read_text() for name in SOURCES}


def provenance():
    return {
        name: hashlib.sha256(text.encode()).hexdigest()
        for name, text in retained_sources().items()
    }


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Mouth source contract changed: " + old[:70])
    result = source.replace(old, new, 1)
    compile(result, "<researched mouth source>", "exec")
    return result


def adapt_face(source, design):
    if design is None:
        return source
    from source_mouth_templates import adapt_mouth_face

    source = adapt_mouth_face(source, True)
    source = once(
        source,
        "return z + lip_detail_depth(x, y)",
        "return z + lip_detail_depth(x, y) + researched_mouth_support(x, y)",
    )
    return (HERE / "mouth_field.py").read_text() + "\n" + source


def adapt_companion(name, source, design):
    if design is None:
        return source
    from source_mouth_templates import adapt_mouth_companions

    source = adapt_mouth_companions(name, source, True)
    if name == "mouth.py" and design["kind"] == "muzzle":
        # Keep the hidden oral bag and tongue behind the original head surface.
        # The outer lip attachment still follows the projected muzzle exactly.
        source = once(
            source,
            "rear_depth = -depth(0, MOUTH_Y) if ring >= 2 else negz",
            "rear_depth = (-depth(0, MOUTH_Y) + researched_mouth_support(0, MOUTH_Y)) if ring >= 2 else negz",
        )
        source = once(
            source,
            "depth(0, MOUTH_Y) - 0.023 + 0.012 * rr * sin(t)",
            "depth(0, MOUTH_Y) - researched_mouth_support(0, MOUTH_Y) - 0.023 + 0.012 * rr * sin(t)",
        )
    return source


def adapt_export_sources(sources, design, blink):
    if design is None:
        return sources
    validate(design)
    if not blink:
        raise ValueError("Researched mouth requires independent blink contract")
    out = dict(sources)
    anchor = '        report["mouth_max_delta_m"] = max(d.length for d in moves)'
    out["pipeline.py"] = once(
        out["pipeline.py"],
        anchor,
        anchor
        + '\n        from researched_mouth_runtime import build_mouth_shapes\n        report["mouth_motion"] = build_mouth_shapes(members[0], semantic_part_ordinals, C)',
    )
    names = [
        "MouthOpen",
        "blink.L",
        "blink.R",
        "MouthRound",
        "MouthSpread",
        "MouthPucker",
    ]
    out["glb_merge.py"] = once(
        out["glb_merge.py"],
        '== ["MouthOpen", "blink.L", "blink.R"]',
        "== " + repr(names),
    )
    out["glb_merge.py"] = once(
        out["glb_merge.py"],
        'node["weights"] = [0.0, 0.0, 0.0]',
        'node["weights"] = ' + repr([0.0] * len(names)),
    )
    return out


def preamble(design):
    if design is None:
        return ""
    runtime = (
        (HERE / "mouth_motion.py").read_text()
        + "\n"
        + (HERE / "mouth_morphs.py").read_text()
    )
    return (
        'import types, sys\n_mouth_runtime=types.ModuleType("researched_mouth_runtime")\nsys.modules["researched_mouth_runtime"]=_mouth_runtime\n'
        + f'exec(compile({runtime!r}, "researched_mouth_runtime", "exec"), _mouth_runtime.__dict__)\n'
    )
