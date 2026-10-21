"""Resolve only owned researched mouth controls and freeze source identity."""

import importlib.util
import json
from pathlib import Path
import sys

LIBRARY = Path(__file__).resolve().parent.parent / "mouth_library"


def resolve_mouth(preset, overrides=None):
    if "design" not in preset:
        if overrides is not None:
            raise ValueError("Mouth controls require a researched preset")
        return None, None
    designs = json.loads((LIBRARY / "presets.json").read_text())
    name = preset["design"]
    if set(preset) != {"design"} or not isinstance(name, str) or name not in designs:
        raise ValueError("Unknown or mixed researched mouth preset")
    if overrides is not None and (
        not isinstance(overrides, dict)
        or set(overrides) - set(designs[name])
        or set(overrides) & {"kind", "version"}
    ):
        raise ValueError("Only owned numeric mouth controls may be overridden")
    old = sys.path[:]
    sys.path.insert(0, str(LIBRARY))
    try:
        spec = importlib.util.spec_from_file_location(
            "component_mouth_source", LIBRARY / "mouth_source.py"
        )
        value = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(value)
        return (
            value.validate({**designs[name], **(overrides or {})}),
            value.provenance(),
        )
    finally:
        sys.path[:] = old
