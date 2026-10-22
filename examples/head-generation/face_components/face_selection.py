"""Resolve researched face choices independently from the other five slots."""

import importlib.util
import json
from pathlib import Path

LIBRARY = Path(__file__).resolve().parent.parent / "face_library"


def module():
    spec = importlib.util.spec_from_file_location("researched_face_design", LIBRARY / "face_design.py")
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def resolve_face(preset, overrides=None):
    name = preset.get("design")
    if name is None:
        if overrides is not None:
            raise ValueError("Face controls require a researched preset")
        return None, None
    if set(preset) != {"design"}:
        raise ValueError("Researched face cannot mix legacy controls or templates")
    presets = json.loads((LIBRARY / "presets.json").read_text())
    if not isinstance(name, str) or name not in presets:
        raise ValueError("Unknown researched face reference")
    m = module()
    if overrides is not None and (not isinstance(overrides, dict) or set(overrides) - set(m.LIMITS)):
        raise ValueError("Only owned numeric face controls can be overridden")
    return m.validate({**presets[name], **(overrides or {})}), m.provenance()
