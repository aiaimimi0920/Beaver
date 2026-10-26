"""Create a native Beaver request for one resolved six-slot composition."""

import argparse
import importlib.util
import json
from pathlib import Path
import re
import sys
from compose import HERE, SURFACE, resolve
from source_layout_bridge import modular_plan
from source_layout.plan_files import write_plan_files


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build_raw_request(
    selection,
    prefix,
    workspace,
    fit_policy="legacy",
    brow_overrides=None,
    nose_overrides=None,
    mouth_overrides=None,
    face_overrides=None,
    ear_overrides=None,
    sampling_overrides=None,
):
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Use a fresh general_face_eyes_combo prefix")
    if (Path(workspace) / prefix).exists():
        raise ValueError("Composition must use a fresh directory")
    snapshot = resolve(
        selection, fit_policy, brow_overrides, nose_overrides, mouth_overrides, face_overrides, ear_overrides, sampling_overrides
    )
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
            mouth_design=snapshot.get("mouth_design"),
            face_design=snapshot.get("face_design"),
            ear_design=snapshot.get("ear_design"),
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


def build_plan(*args, source_seed=False, **kwargs):
    prefix = kwargs.get("prefix", args[1] if len(args) > 1 else None)
    return modular_plan(build_raw_request(*args, **kwargs), prefix, source_seed=source_seed)


def build_request(*args, **kwargs):
    """First native job; callers must retain build_plan's source packaging plan."""
    if kwargs.get("source_seed", False):
        raise ValueError("Source seed requires build_plan and finalize_seed_plan, not build_request")
    return build_plan(*args, **kwargs)["generation"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument(
        "--fit-policy", choices=("legacy", "skin-seated-075"), default="legacy"
    )
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-seed", action="store_true", help="Emit a seed request and complete plan; finalize after native seed succeeds")
    parser.add_argument(
        "--brow-controls", type=Path, help="Bounded researched-brow overrides JSON"
    )
    parser.add_argument(
        "--nose-controls", type=Path, help="Bounded researched-nose overrides JSON"
    )
    parser.add_argument(
        "--mouth-controls", type=Path, help="Bounded researched-mouth overrides JSON"
    )
    parser.add_argument("--face-controls", type=Path, help="Bounded researched-face overrides JSON")
    parser.add_argument("--ear-controls", type=Path, help="Bounded researched-ear overrides JSON")
    parser.add_argument("--sampling-controls", type=Path, help="Explicit first-pass lash sampling JSON")
    args = parser.parse_args()
    target = args.output.resolve()
    if HERE.parent.resolve() in target.parents:
        raise ValueError("Request output must be outside retained recipe directories")
    plan = build_plan(
        json.loads(args.selection.read_text()),
        args.prefix,
        args.workspace,
        args.fit_policy,
        (json.loads(args.brow_controls.read_text()) if args.brow_controls else None),
        (json.loads(args.nose_controls.read_text()) if args.nose_controls else None),
        (json.loads(args.mouth_controls.read_text()) if args.mouth_controls else None),
        (json.loads(args.face_controls.read_text()) if args.face_controls else None),
        (json.loads(args.ear_controls.read_text()) if args.ear_controls else None),
        (json.loads(args.sampling_controls.read_text()) if args.sampling_controls else None),
        source_seed=args.source_seed,
    )
    for kind, path in write_plan_files(plan, args.output).items():
        print(f"{kind}: {path}")
