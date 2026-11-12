#!/usr/bin/env python3
"""One-command safe extraction and append-only patch. Output directory must be new."""

import argparse, pathlib, subprocess, sys
from position_bridge import (
    SOURCE_SHA,
    Rejected,
    require,
    file_sha,
    exclusive_bytes,
    exclusive_json,
    load_baseline,
    check_extraction,
    patch_bytes,
)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--blender", default="blender", help="Trusted Blender 4.3.2 executable"
    )
    p.add_argument("--blend", type=pathlib.Path, required=True)
    p.add_argument(
        "--baseline-dir",
        type=pathlib.Path,
        help="Private pinned baseline data; defaults to adjacent baseline/",
    )
    p.add_argument("--source", type=pathlib.Path, required=True)
    p.add_argument("--output-dir", type=pathlib.Path, required=True)
    a = p.parse_args()
    a.blend = a.blend.resolve()
    a.source = a.source.resolve()
    a.output_dir = a.output_dir.resolve()
    require(file_sha(a.source) == SOURCE_SHA, "Wrong authoritative source SHA256")
    require(
        not a.output_dir.exists(),
        "Output directory already exists; no overwrite permitted",
    )
    original_blend = file_sha(a.blend)
    a.output_dir.mkdir(parents=False, exist_ok=False)
    script = pathlib.Path(__file__).resolve().parent / "extract_blend.py"
    command = [
        a.blender,
        "--background",
        "--factory-startup",
        "--disable-autoexec",
        "--python-exit-code",
        "1",
        "--python",
        str(script),
        "--",
        str(a.blend),
        str(a.output_dir / "extraction"),
    ]
    if a.baseline_dir is not None:
        command.extend(["--baseline-dir", str(a.baseline_dir.resolve())])
    run = subprocess.run(
        command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    exclusive_bytes(a.output_dir / "blender-extraction.log", run.stdout.encode())
    require(
        run.returncode == 0,
        "Blender extraction rejected input. See blender-extraction.log; no GLB written.",
    )
    info, arrays, manifest = load_baseline(a.baseline_dir)
    positions, extraction = check_extraction(
        a.output_dir / "extraction", info, arrays, manifest
    )
    output, report = patch_bytes(a.source.read_bytes(), positions, a.baseline_dir)
    require(
        file_sha(a.blend) == original_blend and file_sha(a.source) == SOURCE_SHA,
        "Source input changed during operation",
    )
    report["extraction_json_sha256"] = file_sha(
        a.output_dir / "extraction/extraction.json"
    )
    exclusive_bytes(a.output_dir / "positions-only.glb", output)
    exclusive_json(a.output_dir / "patch-report.json", report)
    print("PASS", a.output_dir / "positions-only.glb", report["output_sha256"])


if __name__ == "__main__":
    main()
