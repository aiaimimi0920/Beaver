#!/usr/bin/env python3
"""Run the repository's real mouth controller with local GLBs in native Godot.

Only the host head-review/display dependencies are stand-ins. The production
controller is copied byte-for-byte, and all morph operations use imported Godot
MeshInstance3D objects. No model or generated project is added to the repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
CORRECTIVE = "MouthOvalMidCorrective"
MARKER = "MOUTH_CONTROLLER_VALIDATION="


def fingerprint(path: Path) -> dict:
    return {"name": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def reorder_glb(source: Path, destination: Path) -> None:
    """Move the corrective to slot zero without altering geometry or other chunks."""
    data = source.read_bytes()
    magic, version, size = struct.unpack_from("<4sII", data)
    if magic != b"glTF" or version != 2 or size != len(data):
        raise ValueError("Expected a complete GLB version 2 file")
    chunks = []
    offset = 12
    while offset < len(data):
        length, kind = struct.unpack_from("<II", data, offset)
        offset += 8
        chunk = data[offset : offset + length]
        if len(chunk) != length:
            raise ValueError("Truncated GLB chunk")
        chunks.append((kind, chunk))
        offset += length
    if not chunks or chunks[0][0] != 0x4E4F534A:
        raise ValueError("Expected JSON as the first GLB chunk")
    document = json.loads(chunks[0][1])
    reordered = {}
    for mesh_index, mesh in enumerate(document["meshes"]):
        names = mesh.get("extras", {}).get("targetNames", [])
        if CORRECTIVE not in names:
            continue
        corrective_index = names.index(CORRECTIVE)
        order = [corrective_index] + [
            i for i in range(len(names)) if i != corrective_index
        ]
        mesh["extras"]["targetNames"] = [names[i] for i in order]
        for primitive in mesh["primitives"]:
            targets = primitive["targets"]
            if len(targets) != len(order):
                raise ValueError("Morph target name and primitive counts differ")
            primitive["targets"] = [targets[i] for i in order]
        if "weights" in mesh:
            mesh["weights"] = [mesh["weights"][i] for i in order]
        reordered[mesh_index] = order
    if not reordered:
        raise ValueError("V15 input has no named corrective to reorder")
    for node in document.get("nodes", []):
        if node.get("mesh") in reordered and "weights" in node:
            node["weights"] = [node["weights"][i] for i in reordered[node["mesh"]]]
    if any(
        channel["target"]["path"] == "weights"
        for animation in document.get("animations", [])
        for channel in animation["channels"]
    ):
        raise ValueError("Reorder fixture does not support animated morph weight tracks")
    encoded = json.dumps(document, separators=(",", ":")).encode("utf-8")
    chunks[0] = (chunks[0][0], encoded + b" " * (-len(encoded) % 4))
    body = b"".join(
        struct.pack("<II", len(chunk), kind) + chunk for kind, chunk in chunks
    )
    destination.write_bytes(struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body)


def create_project(directory: Path, controller: Path, v15: Path) -> Path:
    extensions = directory / "showcase_extensions"
    extensions.mkdir()
    for name in [
        "assertions.gd",
        "controller_cases.gd",
        "validate_controller.gd",
        "project.godot",
    ]:
        shutil.copyfile(HERE / name, directory / name)
    shutil.copyfile(HERE / "head_review_stub.gd", extensions / "head_review.gd")
    copied_controller = extensions / "researched_mouth_review.gd"
    shutil.copyfile(controller, copied_controller)
    if copied_controller.read_bytes() != controller.read_bytes():
        raise ValueError("Copied controller differs from production source")
    reordered = directory / "v15-reordered.glb"
    reorder_glb(v15, reordered)
    return reordered


def run(args: argparse.Namespace) -> tuple[dict, str]:
    for path in [args.v15, args.v11, args.controller]:
        if not path.is_file():
            raise FileNotFoundError(path)
    with tempfile.TemporaryDirectory(prefix="mouth-controller-validation-") as temporary:
        project = Path(temporary)
        reordered = create_project(project, args.controller, args.v15)
        environment = os.environ.copy()
        environment.update(
            {
                "HOME": str(project),
                "XDG_CACHE_HOME": str(project / "cache"),
                "XDG_DATA_HOME": str(project / "data"),
                "XDG_CONFIG_HOME": str(project / "config"),
                "GODOT_SILENCE_ROOT_WARNING": "1",
            }
        )
        command = [
            args.godot,
            "--headless",
            "--path",
            str(project),
            "--script",
            "res://validate_controller.gd",
            "--",
            str(args.v15),
            str(args.v11),
            str(reordered),
        ]
        process = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=args.timeout,
            env=environment,
            check=False,
        )
        log = process.stdout
        reports = [
            line[len(MARKER) :]
            for line in log.splitlines()
            if line.startswith(MARKER)
        ]
        report = (
            json.loads(reports[-1])
            if reports
            else {
                "passed": False,
                "failures": ["Godot did not produce a validation report"],
            }
        )
        errors = [
            line for line in log.splitlines()
            if "SCRIPT ERROR:" in line or line.startswith("ERROR:")
        ]
        if process.returncode != 0 or errors:
            report["passed"] = False
        report.update(
            {
                "godot_exit_code": process.returncode,
                "godot_errors": errors,
                "production_controller": fingerprint(args.controller),
                "models": {
                    "v15": fingerprint(args.v15),
                    "v11": fingerprint(args.v11),
                    "v15_reordered": fingerprint(reordered),
                },
                "host": (
                    "Minimal head-review and display-sync stand-ins; "
                    "not full NPR application acceptance"
                ),
            }
        )
        return report, log


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--v15", type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument("--v11", type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument("--godot", default="godot", help="Godot 4 executable")
    parser.add_argument(
        "--controller",
        type=lambda value: Path(value).resolve(),
        default=HERE.parent / "runtime" / "mouth_review.gd",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="Optional JSON report; engine log is written beside it with .log suffix",
    )
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
    report, log = run(args)
    output = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(output, encoding="utf-8")
        args.report.with_suffix(".log").write_text(log, encoding="utf-8")
    print(output, end="")
    if not report["passed"]:
        print(log)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
