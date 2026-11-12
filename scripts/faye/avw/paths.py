"""Explicit data/output roots; never silently overwrite the historical candidate."""

import os
from pathlib import Path


def validation_root():
    value = os.environ.get("FAYE_VALIDATION_ROOT")
    if not value:
        raise RuntimeError("Use run.py with --validation-root and --output")
    return Path(value).resolve(strict=True)


def candidate_root():
    value = os.environ.get("FAYE_OUTPUT_ROOT")
    if not value:
        raise RuntimeError("Use run.py with --validation-root and --output")
    result = Path(value).resolve(strict=True)
    source = validation_root()
    if result == source or result.is_relative_to(source):
        raise RuntimeError("Output must be outside validation input data")
    if not (result / ".avw-isolated-run").is_file():
        raise RuntimeError("Output was not initialized by run.py")
    return result
