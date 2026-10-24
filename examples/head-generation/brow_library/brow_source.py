"""Scoped replacement of the brow-only tail in build_eyes, no baseline writes."""

import ast
import hashlib
from pathlib import Path
from brow_design import validate
from brow_shapes import brow_polygons, brow_area, brow_sampling_divisions

HERE = Path(__file__).resolve().parent


def validate_geometry_style(design, style):
    if design is None:
        if style.get("sampling_brow_divisions", 0):
            raise ValueError("Brow sampling requires a researched brow design")
        return None
    c = style["calibration"]
    span = c["height_m"] * (c["brow_outer_x"] - c["brow_inner_x"])
    if span <= 0:
        raise ValueError("Brow local span must be positive")
    polygons = brow_polygons(design, span, c["height_m"] * c["brow_thickness"], brow_sampling_divisions(design, style))
    if any(
        not -0.05 * span <= x <= 1.05 * span or abs(y) > 0.5 * span
        for polygon in polygons for x, y in polygon
    ):
        raise ValueError("Brow design exceeds local shape envelope")
    return {
        "islands_per_side": len(polygons),
        "local_area_m2": sum(brow_area(p) for p in polygons),
    }


def retained_sources():
    names = ("brow_design.py", "brow_shapes.py", "brow_mesh.py", "brow_source.py")
    return {name: (HERE / name).read_text() for name in names}


def runtime_source():
    return (HERE / "brow_shapes.py").read_text() + "\n" + (HERE / "brow_mesh.py").read_text()


def adapt(source, design):
    design = validate(design)
    if design is None:
        return source
    tree = ast.parse(source)
    builders = [
        n for n in tree.body
        if isinstance(n, ast.FunctionDef) and n.name == "build_eyes"
    ]
    if len(builders) != 1:
        raise ValueError("Missing unique eye assembly for brow design")
    starts = [
        n for n in ast.walk(builders[0])
        if isinstance(n, ast.Assign)
        and any(isinstance(t, ast.Name) and t.id == "brow" for t in n.targets)
    ]
    if len(starts) != 1 or not isinstance(starts[0].value, ast.List) or starts[0].value.elts:
        raise ValueError("Brow assembly source contract changed")
    lines = source.splitlines(keepends=True)
    tail = "".join(lines[starts[0].lineno - 1:builders[0].end_lineno])
    if tail.count('"Tapered brow "') != 1 or "grid_faces(2, 25, side < 0)" not in tail:
        raise ValueError("Unexpected retained brow assembly")
    replacement = (
        "        from researched_brow_runtime import build_designed_brow\n"
        "        build_designed_brow(side, STYLE['brow_design'])\n"
    )
    result = (
        "".join(lines[:starts[0].lineno - 1])
        + replacement
        + "".join(lines[builders[0].end_lineno:])
    )
    compile(result, "<research brow grammar>", "exec")
    return result


def provenance():
    return {name: hashlib.sha256(text.encode()).hexdigest() for name, text in retained_sources().items()}
