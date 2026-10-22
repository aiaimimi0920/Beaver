"""Retain packager evidence and source snapshots within declared native outputs."""

import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
EYES = HERE.parent / "semantic_eyes"


def append_artifacts(request, prefix, workspace, data, report, nose_sources, mouth_sources, face_sources, ear_sources=None):
    artifacts = {
        f"facial_surface_{path.name}": path.read_text()
        for path in HERE.glob("*.py")
        if not path.name.startswith("test_") and not path.name.startswith("source_")
    }
    for name in ("fit.py", "curve_profile.py", "diagnostics.py"):
        artifacts["coherent_dependency_" + name] = (
            HERE.parent / "coherent_surface" / name
        ).read_text()
    artifacts["facial_surface_preflight.json"] = json.dumps(
        dict(
            data=data,
            report=report,
            nose_library_sources=nose_sources,
            face_library_sources=face_sources,
            mouth_library_sources=mouth_sources,
            ear_library_sources=ear_sources or {},
            packager_helpers={
                **{path.name: path.read_text() for path in HERE.glob("source_*.py")},
                "semantic_illustrated_art.py": (
                    EYES / "illustrated_art.py"
                ).read_text(),
            },
        ),
        indent=2,
    )
    artifacts["facial_surface_source_manifest.json"] = json.dumps(
        {
            name: hashlib.sha256(content.encode()).hexdigest()
            for name, content in artifacts.items()
        },
        indent=2,
    )
    for name, content in artifacts.items():
        path = prefix + name
        request["code"] += f"Path(beaver_output({path!r})).write_text({content!r})\n"
        previous = Path(workspace) / path
        request["outputs"].append(
            dict(
                path=path,
                expectedSha256=(
                    hashlib.sha256(previous.read_bytes()).hexdigest()
                    if previous.is_file()
                    else None
                ),
            )
        )
