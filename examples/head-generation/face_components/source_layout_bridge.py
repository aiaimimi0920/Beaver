"""Load the uniquely named shared source-layout package."""

from pathlib import Path
import sys

_parent = str(Path(__file__).resolve().parent.parent)
_previous_path = sys.path[:]
try:
    sys.path.insert(0, _parent)
    from source_layout.package import modular_plan, packaging_request
    from source_layout.evidence import verify_packaging, native_job
finally:
    sys.path[:] = _previous_path
