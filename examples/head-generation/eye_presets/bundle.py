"""Select an explicit eye preset and package it; never run Blender here."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
SURFACE = HERE.parent / "facial_surface"


def load_preset(preset_id):
    if preset_id not in {
        "almond-balanced-796",
        "slender-flat-797",
        "soft-downturned-798",
        "sharp-upturned-799",
        "flat-upper-round-lower-800",
        "arched-upper-flat-lower-801",
        "angular-upper-802",
        "almond-taper-experiment-803",
        "rounded-arch-shallow-805",
        "asymmetric-angular-807",
        "soft-wing-downturned-808",
        "clean-angular-811",
        "clean-downturned-810",
        "clean-slender-812",
        "face-jaw-wide-817",
        "face-jaw-narrow-818",
        "face-wide-819",
        "face-chin-narrow-820",
    }:
        raise ValueError("Unknown explicit eye preset")
    spec = json.loads((HERE / (preset_id + ".json")).read_text())
    for name, expected in spec["baseline_files_sha256"].items():
        if hashlib.sha256((SURFACE / name).read_bytes()).hexdigest() != expected:
            raise ValueError("Preserved 795 baseline changed: " + name)
    if preset_id in {"face-jaw-wide-817", "face-jaw-narrow-818", "face-wide-819", "face-chin-narrow-820"}:
        from face_evidence import validate_fixed_combination
        validate_fixed_combination(spec)
    return spec


def build_request(preset_id, prefix, workspace):
    spec = load_preset(preset_id)
    if prefix in {
        "assets/aster/general_face_eyes_795/",
        "assets/aster/general_face_eyes_796/",
    }:
        raise ValueError("Never overwrite a preserved baseline")
    previous_path = sys.path[:]
    sys.path.insert(0, str(SURFACE))
    try:
        module_spec = importlib.util.spec_from_file_location(
            "preset_surface_packager", SURFACE / "bundle.py"
        )
        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)
        return module.build_request(
            spec["surface_parameters"],
            spec["eye_parameters"],
            prefix,
            workspace,
            spec["reviewed_support_bound_m"],
            spec["style_profile"],
            blink=spec["blink"],
            **({"silhouette_parameters": spec["silhouette_parameters"]}
               if "silhouette_parameters" in spec else {}),
        )
    finally:
        sys.path[:] = previous_path


def validate_output_path(output):
    # A packaging destination must not overwrite any retained recipe/source file.
    target = Path(output).resolve()
    recipes = HERE.parent.resolve()
    if target == recipes or recipes in target.parents:
        raise ValueError("Output must be outside retained recipe directories")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preset", required=True)
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    validate_output_path(args.output)
    args.output.write_text(
        json.dumps(build_request(args.preset, args.prefix, args.workspace))
    )
