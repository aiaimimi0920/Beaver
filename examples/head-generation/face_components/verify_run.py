"""Read-only artifact-chain verification; never infer aesthetic approval."""

import hashlib
import importlib.util
import json
from pathlib import Path
import re

from compose import HERE, verify_snapshot


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def resource(root, relative):
    path = (root / relative).resolve()
    if (
        not isinstance(relative, str)
        or Path(relative).is_absolute()
        or not path.is_relative_to(root)
    ):
        raise ValueError("Artifact escapes workspace")
    return path


class NativeJobFailure(ValueError):
    def __init__(self, stage, status):
        super().__init__(stage + ": " + status)
        self.status = stage + "_failed" if status == "failed" else "incomplete"


def receipt(path, root, stage):
    job = json.loads(Path(path).read_text())["response"]["result"]["result"]
    if (
        job["status"] != "succeeded"
        or job["result"]["success"] is not True
        or job["result"]["exitCode"] != 0
    ):
        raise NativeJobFailure(stage, job["status"])
    if Path(job["process"]["workspace"]).resolve() != root:
        raise ValueError("Receipt workspace mismatch")
    rows = job["result"]["outputs"]
    outputs = {row["path"]: row["sha256"] for row in rows}
    if len(outputs) != len(rows):
        raise ValueError("Duplicate receipt outputs")
    return job, outputs


def verify_run(spec):
    root = Path(spec["workspace"]).resolve()
    prefix = spec["prefix"]
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Invalid composition prefix")
    gen, generated = receipt(spec["generation_receipt"], root, "generation")
    sdf, baked = receipt(spec["sdf_receipt"], root, "sdf")
    immutable = [
        prefix + name
        for name in (
            "aster_head.glb",
            "aster_head_runtime_morph.blend",
            "semantic_parameters.json",
        )
    ]
    required_baked = [
        prefix + name
        for name in (
            "generation_report.json",
            "aster_definition.tres",
            "sdf/face_sdf360.png",
            "sdf/bake_report.json",
            "sdf/generation_binding.json",
            "sdf/input_hashes.json",
        )
    ]
    if any(name not in generated for name in immutable) or any(
        name not in baked for name in required_baked
    ):
        raise ValueError("Required native outputs missing")
    if any(name in baked for name in immutable):
        raise ValueError("SDF unexpectedly replaces immutable geometry")
    for name, digest in {**generated, **baked}.items():
        if not name.startswith(prefix) or sha(resource(root, name)) != digest:
            raise ValueError("Changed or misplaced native output: " + name)
    inputs = json.loads(resource(root, prefix + "sdf/input_hashes.json").read_text())
    for name in immutable + [
        prefix + "generation_report.json",
        prefix + "aster_definition.tres",
    ]:
        if inputs.get(name) != generated.get(name) or name not in generated:
            raise ValueError("SDF input differs from generated artifact: " + name)
    binding = json.loads(
        resource(root, prefix + "sdf/generation_binding.json").read_text()
    )
    expected = {name: generated[name] for name in immutable}
    if (
        binding["receipt_sha256"] != sha(spec["generation_receipt"])
        or binding["job_id"] != gen["jobId"]
        or binding["script_sha256"] != gen["process"]["scriptSha256"]
        or binding["immutable_outputs"] != expected
    ):
        raise ValueError("SDF binding differs from generation job")
    report = json.loads(resource(root, prefix + "generation_report.json").read_text())
    snapshot = verify_snapshot(report["face_components"])
    if (
        snapshot["selection"] != spec["selection"]
        or snapshot.get("fit_policy", "legacy") != spec["fit_policy"]
    ):
        raise ValueError("Requested selection or fit policy mismatch")
    parameters = json.loads(
        resource(root, prefix + "semantic_parameters.json").read_text()
    )
    historical_defaults = {
        "aperture_upper_share": 0.0,
        "aperture_corner_taper": 0.0,
        "upper_corner_softness": 0.0,
        "upper_angularity": 0.0,
        "wing_lift_scale": 1.0,
        "lateral_guide_regularization": 1.0,
        "lateral_guide_inward_power": 0.0,
    }
    normalized = dict(snapshot["eye_parameters"])
    for key in parameters.keys() - normalized.keys():
        if key not in historical_defaults:
            raise ValueError("Unrecognized historical parameter default: " + key)
        normalized[key] = historical_defaults[key]
    if parameters != normalized or report["semantic_eyes"]["parameters"] != parameters:
        raise ValueError("Snapshot/geometry parameter mismatch")
    actual_sdf = report["semantic_eyes"]["sdf"]
    if actual_sdf["policy"] != snapshot["shading_policy"]:
        raise ValueError("SDF policy mismatch")
    if (
        actual_sdf["generation_binding"] != binding
        or actual_sdf["atlas_sha256"] != baked[prefix + "sdf/face_sdf360.png"]
    ):
        raise ValueError("Final report SDF binding mismatch")
    capture_verifier = load_capture_verifier()
    captures = {
        profile: capture_verifier(
            root,
            spec[profile + "_manifest"],
            "res://" + prefix + "aster_head.glb",
            profile,
        )
        for profile in ("eye", "mouth")
    }
    return {
        "status": "technically_verified",
        "selection": spec["selection"],
        "fit_policy": spec["fit_policy"],
        "composition_sha256": snapshot["composition_sha256"],
        "glb_sha256": generated[immutable[0]],
        "generation_job": gen["jobId"],
        "sdf_job": sdf["jobId"],
        "captures": captures,
        "scope": "Current artifact integrity and recorded native states, not aesthetic approval, current-generator reproducibility or untested combinations",
    }


def load_capture_verifier():
    module_spec = importlib.util.spec_from_file_location(
        "run_capture_verifier", HERE.parent / "eye_presets/verify_capture.py"
    )
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    return module.verify_capture
