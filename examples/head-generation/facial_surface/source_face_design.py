"""Integrate researched face calibration without rewriting retained recipes."""

import importlib.util
from pathlib import Path


def module():
    path = Path(__file__).resolve().parent.parent / "face_library/face_design.py"
    spec = importlib.util.spec_from_file_location("surface_face_design", path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def apply(style, design, silhouette, templates):
    if design is not None and (templates or any((silhouette or {}).values())):
        raise ValueError("Do not mix researched face and legacy silhouette controls")
    return module().apply_design(style, design)


def adapt_sources(snapshot, retained_stations):
    from source_family_templates import instrument_ear_constraints, transport_ear_constraints
    from source_outline_trace import instrument_outline
    from source_outline_fairing import adapt_outline_fairing
    shell = snapshot / "shell.py"
    from source_mouth_templates import replace_once
    source = adapt_outline_fairing(instrument_outline(shell.read_text()))
    import inspect
    source = replace_once(
        source, "from face import *\n",
        "from face import *\n\n" + inspect.getsource(module().researched_underside_lift) + "\n",
    )
    source = replace_once(
        source,
        'return Y0 + H * (lift + C["chin_return_lift"] * q * q - drop)',
        'return Y0 + H * (lift + researched_underside_lift(C, lift) * q * q - drop)',
    )
    shell.write_text(source)
    boundary = snapshot / "ear_boundary.py"
    boundary.write_text(transport_ear_constraints(instrument_ear_constraints(boundary.read_text()), retained_stations))
