"""Apply researched ear calibration and source changes to fresh native outputs."""

import importlib.util
from pathlib import Path

LIBRARY = Path(__file__).resolve().parent.parent / "ear_library"


def module(name):
    spec = importlib.util.spec_from_file_location("surface_" + name, LIBRARY / (name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def apply(style, design, components):
    if design is not None and any(k.startswith("ear_") for k in (components or {})):
        raise ValueError("Do not mix researched ear and legacy ear controls")
    return module("ear_design").apply_design(style, design)


def adapt(source, design):
    return module("ear_profile").adapt(source, design)


def adapt_boundary(source, design):
    return module("ear_profile").adapt_boundary(source, design)


def adapt_topology(source, design):
    return module("ear_profile").adapt_topology(source, design)

