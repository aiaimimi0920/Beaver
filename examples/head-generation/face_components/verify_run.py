"""Read-only artifact-chain verification; never infer aesthetic approval."""

import hashlib
import importlib.util
import json
from pathlib import Path
import re

from compose import HERE, verify_snapshot
from verify_outline import verify_outline_guard
from brow_evidence import verify_brow_artifacts
from nose_evidence import verify_nose_artifacts
from mouth_evidence import verify_mouth_artifacts
from face_evidence import verify_face_artifacts
from ear_evidence import verify_ear_artifacts
from source_layout_bridge import verify_packaging


def verify_requested_composition(snapshot, spec):
    """Bind adjustable designs to the saved pre-launch composition, not a label."""
    if (
        snapshot["selection"] != spec["selection"]
        or snapshot.get("fit_policy", "legacy") != spec["fit_policy"]
    ):
        raise ValueError("Requested selection or fit policy mismatch")
    expected = spec.get("composition_sha256")
    if (
        snapshot.get("nose_design") is not None
        or snapshot.get("mouth_design") is not None
        or snapshot.get("face_design") is not None
        or snapshot.get("ear_design") is not None
    ) and expected is None:
        raise ValueError("Researched design requires requested composition identity")
    if expected is not None and expected != snapshot.get("composition_sha256"):
        raise ValueError("Requested composition identity mismatch")


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
    if Path(job["process"]["workspace"]).resolve() != root:
        raise ValueError("Receipt workspace mismatch")
    if (
        job["status"] != "succeeded"
        or job["result"]["success"] is not True
        or job["result"]["exitCode"] != 0
    ):
        raise NativeJobFailure(stage, job["status"])
    rows = job["result"]["outputs"]
    outputs = {row["path"]: row["sha256"] for row in rows}
    if len(outputs) != len(rows):
        raise ValueError("Duplicate receipt outputs")
    return job, outputs


def verify_template_reports(snapshot, surface, calibration):
    templates = snapshot.get("family_templates", [])
    if not isinstance(templates, list):
        raise ValueError("Invalid template snapshot")
    seen = set()
    fields = {
        "face": "family_template",
        "mouth": "mouth_template",
        "nose": "nose_template",
    }
    for template in templates:
        slot = template.get("slot")
        if slot not in fields or slot in seen:
            raise ValueError("Duplicate or unknown template slot")
        seen.add(slot)
        record = surface.get(fields[slot], {})
        if record.get("template") != template:
            raise ValueError("Snapshot/template report mismatch")
        if slot == "face" and record.get("final_width_stations") != calibration.get(
            "width_stations"
        ):
            raise ValueError("Template/final width calibration mismatch")
        if slot == "mouth" and (
            calibration.get("mouth_seam_bow_height") != template["seam_bow_height"]
            or calibration.get("mouth_seam_shoulder_radius_ratio")
            != template["shoulder_radius_ratio"]
        ):
            raise ValueError("Template/mouth calibration mismatch")
        if slot == "nose" and (
            calibration.get("nose_stations") != template["nose_stations"]
            or calibration.get("nose_template_mode") != template["cross_section"]
        ):
            raise ValueError("Template/nose calibration mismatch")
    if any(field in surface and slot not in seen for slot, field in fields.items()):
        raise ValueError("Unexpected generated template report")


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
    verify_packaging(
        report,
        root,
        prefix,
        spec["generation_receipt"],
        spec.get("source_packaging_receipt"),
    )
    snapshot = verify_snapshot(report["face_components"])
    verify_brow_artifacts(snapshot, report, root, prefix, generated)
    verify_nose_artifacts(snapshot, report, root, prefix, generated)
    verify_mouth_artifacts(snapshot, report, root, prefix, generated)
    verify_face_artifacts(snapshot, report, root, prefix, generated)
    verify_ear_artifacts(snapshot, report, root, prefix, generated)
    verify_outline_guard(snapshot, report)
    verify_template_reports(
        snapshot,
        report.get("facial_surface", {}),
        (
            json.loads(resource(root, prefix + "face_style.json").read_text())[
                "calibration"
            ]
            if snapshot.get("family_templates")
            else {}
        ),
    )
    verify_requested_composition(snapshot, spec)
    parameters = json.loads(
        resource(root, prefix + "semantic_parameters.json").read_text()
    )
    historical_defaults = {
        "aperture_upper_share": 0.0,
        "aperture_corner_taper": 0.0,
        "upper_corner_softness": 0.0,
        "upper_angularity": 0.0,
        "wing_lift_scale": 1.0,
        "lateral_aperture_follow": 0.0,
        "lateral_blink_follow": 0.85,
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
    if snapshot.get("mouth_design") is not None:
        import sys

        library = HERE.parent / "mouth_library"
        sys.path.insert(0, str(library))
        module_spec = importlib.util.spec_from_file_location(
            "mouth_motion_capture_verifier", library / "verify_capture.py"
        )
        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)
        captures["mouth_motion"] = module.verify_capture(
            root, spec["mouth_motion_manifest"], "res://" + prefix + "aster_head.glb",
            spec.get("mouth_motion_probe_profile", "mouth"),
        )
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
