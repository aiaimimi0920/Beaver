"""Bind SDF to the actual generated composition, not a re-resolved catalog."""

import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path
import sys
from compose import HERE, verify_snapshot
from brow_evidence import verify_brow_artifacts
from nose_evidence import verify_nose_artifacts
from mouth_evidence import verify_mouth_artifacts
from face_evidence import verify_face_artifacts
from ear_evidence import verify_ear_artifacts
from source_layout_bridge import verify_packaging


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def bind_composition_report(report_path, prefix, receipt):
    raw = report_path.read_bytes()
    job = json.loads(Path(receipt).read_text())["response"]["result"]["result"]
    if job["status"] != "succeeded" or job["result"]["success"] is not True:
        raise ValueError("Successful generation receipt required")
    outputs = job["result"]["outputs"]
    hashes = {row["path"]: row["sha256"] for row in outputs}
    if len(hashes) != len(outputs):
        raise ValueError("Duplicate receipt output paths")
    if hashes.get(prefix + "generation_report.json") != hashlib.sha256(raw).hexdigest():
        raise ValueError("Composition report differs from native generation receipt")
    return json.loads(raw)


def build_request(prefix, workspace, receipt, source_packaging_receipt=None):
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Expected composition output prefix")
    root = Path(workspace)
    report = bind_composition_report(
        root / prefix / "generation_report.json", prefix, receipt
    )
    verify_packaging(report, root, prefix, receipt, source_packaging_receipt)
    snapshot = verify_snapshot(report.get("face_components"))
    generation = json.loads(Path(receipt).read_text())["response"]["result"]["result"]
    generated = {row["path"]: row["sha256"] for row in generation["result"]["outputs"]}
    verify_brow_artifacts(snapshot, report, root, prefix, generated)
    verify_nose_artifacts(snapshot, report, root, prefix, generated)
    verify_mouth_artifacts(snapshot, report, root, prefix, generated)
    verify_face_artifacts(snapshot, report, root, prefix, generated)
    verify_ear_artifacts(snapshot, report, root, prefix, generated)
    surface = report["facial_surface"]
    for key in ("style_profile", "reviewed_support_bound_m"):
        if surface[key] != snapshot[key]:
            raise ValueError("Composition/generation mismatch: " + key)
    eyes = HERE.parent / "semantic_eyes"
    old_path = sys.path[:]
    # semantic sdf_bundle imports the sibling generic name 'bundle'.
    previous_bundle = sys.modules.pop("bundle", None)
    sys.path.insert(0, str(eyes))
    try:
        module = load_module(eyes / "sdf_bundle.py", "component_sdf_packager")
        return module.build_request(
            snapshot["eye_parameters"],
            prefix,
            root,
            receipt,
            snapshot["shading_policy"],
        )
    finally:
        sys.path[:] = old_path
        sys.modules.pop("bundle", None)
        if previous_bundle is not None:
            sys.modules["bundle"] = previous_bundle


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--generation-receipt", type=Path, required=True)
    parser.add_argument("--source-packaging-receipt", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if HERE.parent.resolve() in args.output.resolve().parents:
        raise ValueError("Request output must be outside retained recipe directories")
    args.output.write_text(
        json.dumps(
            build_request(
                args.prefix,
                args.workspace,
                args.generation_receipt,
                args.source_packaging_receipt,
            )
        )
    )
