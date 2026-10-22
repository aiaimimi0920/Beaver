"""Explicit library dispatch; no change to the retained mouth recipes."""

import importlib.util
from pathlib import Path
import sys

LIBRARY = Path(__file__).resolve().parent.parent / "mouth_library"


def module():
    old = sys.path[:]
    sys.path.insert(0, str(LIBRARY))
    try:
        spec = importlib.util.spec_from_file_location(
            "researched_mouth_source", LIBRARY / "mouth_source.py"
        )
        value = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(value)
        return value
    finally:
        sys.path[:] = old


def apply(style, design, lips, components, templates):
    if design is not None and (
        lips or templates or "mouth_width_scale" in (components or {})
    ):
        raise ValueError(
            "Cannot mix researched mouth and legacy mouth controls/templates"
        )
    return module().apply(style, design)
