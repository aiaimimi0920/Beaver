"""Resolve researched lateral ears independently of the other five slots."""

import importlib.util
import json
from pathlib import Path

LIBRARY = Path(__file__).resolve().parent.parent / "ear_library"


def module():
    spec = importlib.util.spec_from_file_location("researched_ear_design", LIBRARY / "ear_design.py")
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def resolve_ear(preset, overrides=None):
    name = preset.get("design")
    if name is None:
        if overrides is not None:
            raise ValueError("Ear controls require a researched preset")
        return None, None
    if set(preset) != {"design"}:
        raise ValueError("Researched ears cannot mix legacy controls")
    presets = json.loads((LIBRARY / "presets.json").read_text())
    if not isinstance(name, str) or name not in presets:
        raise ValueError("Unknown researched ear reference")
    m = module()
    if overrides is not None and (not isinstance(overrides, dict) or set(overrides) - set(m.LIMITS)):
        raise ValueError("Only owned numeric ear controls can be overridden")
    return m.validate({**presets[name], **(overrides or {})}), m.provenance()
