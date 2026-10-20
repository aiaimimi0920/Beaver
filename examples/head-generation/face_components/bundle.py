"""Create a native Beaver request for one resolved six-slot composition."""

import argparse
import importlib.util
import json
from pathlib import Path
import re
import sys
from compose import HERE, SURFACE, resolve


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build_request(
    selection,
    prefix,
    workspace,
    fit_policy="legacy",
    brow_overrides=None,
    nose_overrides=None,
):
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Use a fresh general_face_eyes_combo prefix")
    if (Path(workspace) / prefix).exists():
        raise ValueError("Composition must use a fresh directory")
    snapshot = resolve(selection, fit_policy, brow_overrides, nose_overrides)
    old_path = sys.path[:]
    sys.path.insert(0, str(SURFACE))
    try:
        packager = load_module(SURFACE / "bundle.py", "component_surface_packager")
        request = packager.build_request(
            snapshot["surface_parameters"],
            snapshot["eye_parameters"],
            prefix,
            workspace,
            snapshot["reviewed_support_bound_m"],
            snapshot["style_profile"],
            blink=snapshot["blink"],
            silhouette_parameters=snapshot["silhouette_parameters"],
            lip_parameters=snapshot["lip_parameters"],
            component_parameters=snapshot["component_parameters"],
            family_templates=snapshot.get("family_templates"),
            brow_design=snapshot.get("brow_design"),
            nose_design=snapshot.get("nose_design"),
        )
    finally:
        sys.path[:] = old_path
    # Reuse existing output, so native output limits remain unchanged.
    request["code"] += f'report["face_components"] = {snapshot!r}\n'
    request["code"] += (
        f'Path(beaver_output({prefix + "generation_report.json"!r})).write_text('
        "json.dumps(report, indent=2))\n"
    )
    compile(request["code"], "<face composition>", "exec")
    return request


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument(
        "--fit-policy", choices=("legacy", "skin-seated-075"), default="legacy"
    )
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--brow-controls", type=Path, help="Bounded researched-brow overrides JSON"
    )
    parser.add_argument(
        "--nose-controls", type=Path, help="Bounded researched-nose overrides JSON"
    )
    args = parser.parse_args()
    target = args.output.resolve()
    if HERE.parent.resolve() in target.parents:
        raise ValueError("Request output must be outside retained recipe directories")
    args.output.write_text(
        json.dumps(
            build_request(
                json.loads(args.selection.read_text()),
                args.prefix,
                args.workspace,
                args.fit_policy,
                (
                    json.loads(args.brow_controls.read_text())
                    if args.brow_controls
                    else None
                ),
                (
                    json.loads(args.nose_controls.read_text())
                    if args.nose_controls
                    else None
                ),
            )
        )
    )
