"""Package integration of the researched nose library, leaving old recipes intact."""

import importlib.util
import sys
from pathlib import Path

LIBRARY = Path(__file__).resolve().parent.parent / "nose_library"


def module():
    old = sys.path[:]
    sys.path.insert(0, str(LIBRARY))
    try:
        spec = importlib.util.spec_from_file_location(
            "researched_nose_source", LIBRARY / "nose_source.py"
        )
        result = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(result)
        return result
    finally:
        sys.path[:] = old


def apply(style, design, components, templates):
    if design is not None and (
        templates or any(k.startswith("nose_") for k in (components or {}))
    ):
        raise ValueError(
            "Do not mix researched nose design and legacy nose controls/templates"
        )
    return module().apply_design(style, design)


def preamble(design):
    if design is None:
        return ""
    source = (LIBRARY / "nose_mesh.py").read_text()
    return (
        'import types, sys\n_nose_runtime=types.ModuleType("researched_nose_runtime")\nsys.modules["researched_nose_runtime"]=_nose_runtime\n'
        + f'exec(compile({source!r}, "researched_nose_runtime", "exec"), _nose_runtime.__dict__)\n'
    )


def retained(design):
    return {} if design is None else module().retained_sources()
