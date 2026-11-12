"""Reproduce AVW in a new isolated directory using externally supplied data."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from band_connections import MODES

INPUTS = [
    "avd-joint-obsolete-crosslinks-20261112/full-fields.npz",
    "avd-joint-obsolete-crosslinks-20261112/full-geometry.npz",
    "avd-joint-obsolete-crosslinks-20261112/region.npz",
    "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz",
    "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz",
    "atd-band-protected-constraint-audit-20261112/SCOPE.json",
    "atd-band-protected-constraint-audit-20261112/CONSTRAINTS.json",
    "atc-current-arc-station-parameters-20261112/CURRENT-ARCS.json",
    "apk-coverage-correction-20261112/fields.npz",
    "apk-coverage-correction-20261112/new.npz",
]
STEPS = [
    "build",
    "internal",
    "interfacefix",
    "eval",
    "interface",
    "finalaudit",
    "internal",
    "edges",
    "bounds",
    "distance",
    "fields_detail",
    "target",
    "camera",
    "trace",
    "check",
]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare(validation, output):
    validation = validation.resolve(strict=True)
    output = output.resolve()
    if output == validation or output.is_relative_to(validation):
        raise ValueError("Output must be outside validation input data")
    missing = [name for name in INPUTS if not (validation / name).is_file()]
    if missing:
        raise FileNotFoundError("Missing input files: " + ", ".join(missing))
    identities = {name: digest(validation / name) for name in INPUTS}
    output.mkdir(parents=True, exist_ok=False)
    (output / ".avw-isolated-run").write_text("AVW reproduction\n")
    return validation, output, identities


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--validation-root", type=Path, required=True)
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="New directory outside validation-root; never overwritten",
    )
    parser.add_argument(
        "--blender", type=Path, help="Optional Blender executable for diagnostic images"
    )
    parser.add_argument("--band-connections", choices=MODES, default="baseline")
    args = parser.parse_args()
    validation, output, hashes = prepare(args.validation_root, args.output)
    env = dict(
        os.environ,
        FAYE_VALIDATION_ROOT=str(validation),
        FAYE_OUTPUT_ROOT=str(output),
        PYTHONIOENCODING="utf-8",
        FAYE_BAND_CONNECTIONS=args.band_connections,
    )
    root = Path(__file__).resolve().parent
    commands = [(step, [sys.executable, str(root / (step + ".py"))]) for step in STEPS]
    if args.blender:
        commands += [
            (
                step,
                [
                    str(args.blender.resolve(strict=True)),
                    "--background",
                    "--factory-startup",
                    "--python-exit-code",
                    "1",
                    "--python",
                    str(root / (step + ".py")),
                ],
            )
            for step in ["render", "close"]
        ]
    report = {
        "inputs": hashes,
        "band_connections": args.band_connections,
        "steps": [],
        "visual_accepted": False,
        "native_generated": False,
        "glb_verified": False,
        "note": "Numerical research reproduction, not production acceptance",
    }
    try:
        for index, (step, command) in enumerate(commands):
            with (output / f"{index:02}-{step}.log").open("w", encoding="utf-8") as log:
                result = subprocess.run(
                    command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT
                )
            report["steps"].append({"step": step, "exit_code": result.returncode})
            print(step, result.returncode, flush=True)
            if result.returncode:
                raise RuntimeError(f"{step} failed; see {output}")
    finally:
        report["inputs_unchanged"] = all(
            digest(validation / n) == h for n, h in hashes.items()
        )
        (output / "RUN.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if not report["inputs_unchanged"]:
        raise RuntimeError("Input changed during run")


if __name__ == "__main__":
    main()
