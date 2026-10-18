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
    if preset_id not in {"almond-balanced-796", "slender-flat-797"}:
        raise ValueError("Unknown explicit eye preset")
    spec = json.loads((HERE / (preset_id + ".json")).read_text())
    for name, expected in spec["baseline_files_sha256"].items():
        if hashlib.sha256((SURFACE / name).read_bytes()).hexdigest() != expected:
            raise ValueError("Preserved 795 baseline changed: " + name)
    return spec


def build_request(preset_id, prefix, workspace):
    spec = load_preset(preset_id)
    if prefix in {"assets/aster/general_face_eyes_795/",
                  "assets/aster/general_face_eyes_796/"}:
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
            spec["surface_parameters"], spec["eye_parameters"], prefix,
            workspace, spec["reviewed_support_bound_m"], spec["style_profile"],
            blink=spec["blink"],
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
    args.output.write_text(json.dumps(build_request(
        args.preset, args.prefix, args.workspace
    )))
