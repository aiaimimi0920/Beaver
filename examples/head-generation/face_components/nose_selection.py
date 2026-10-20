"""Resolve researched nose presets and bind source identity to a selection."""

import importlib.util
import json
import sys
from pathlib import Path

LIBRARY = Path(__file__).resolve().parent.parent / "nose_library"


def resolve_nose(preset, overrides=None):
    design = preset.get("design")
    if design is None:
        if overrides is not None:
            raise ValueError("Nose controls require a researched preset")
        return None, None
    if set(preset) != {"design"}:
        raise ValueError("Researched nose cannot mix legacy controls or templates")
    presets = json.loads((LIBRARY / "presets.json").read_text())
    if not isinstance(design, str) or design not in presets:
        raise ValueError("Unknown researched nose reference")
    old = sys.path[:]
    sys.path.insert(0, str(LIBRARY))
    try:
        spec = importlib.util.spec_from_file_location(
            "component_nose_source", LIBRARY / "nose_source.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        if overrides is not None and (
            not isinstance(overrides, dict)
            or set(overrides) - set(presets[design])
            or set(overrides) & {"version", "kind"}
        ):
            raise ValueError("Only owned numeric nose controls can be overridden")
        return (
            module.validate({**presets[design], **(overrides or {})}),
            module.provenance(),
        )
    finally:
        sys.path[:] = old
