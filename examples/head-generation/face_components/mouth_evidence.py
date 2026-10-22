"""Bind new mouth design, actual GLB targets and sampled-pose evidence."""

import hashlib
import importlib.util
import json
import math
from pathlib import Path
from mouth_selection import LIBRARY

SOURCES = {
    "mouth_design.py",
    "mouth_field.py",
    "mouth_motion.py",
    "mouth_morphs.py",
    "mouth_source.py",
}
NAMES = ["MouthOpen", "blink.L", "blink.R", "MouthRound", "MouthSpread", "MouthPucker"]


def validate_report(snapshot, report, style, frozen):
    design = snapshot.get("mouth_design")
    record = report.get("mouth_design")
    if design is None:
        if (
            record is not None
            or style.get("mouth_design") is not None
            or report.get("mouth_motion") is not None
        ):
            raise ValueError("Unexpected unbound researched mouth")
        return False
    spec = importlib.util.spec_from_file_location(
        "mouth_evidence_design_v1", LIBRARY / "mouth_design.py"
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.validate(design)
    c = style["calibration"]
    expected = snapshot.get("mouth_source_sha256")
    if (
        not isinstance(expected, dict)
        or set(expected) != SOURCES
        or not isinstance(frozen, dict)
        or set(frozen) != SOURCES
    ):
        raise ValueError("Missing mouth source identity")
    if any(
        not isinstance(s, str) or hashlib.sha256(s.encode()).hexdigest() != expected[k]
        for k, s in frozen.items()
    ):
        raise ValueError("Frozen mouth source differs")
    if (
        not isinstance(record, dict)
        or record.get("parameters") != design
        or record.get("source_sha256") != expected
    ):
        raise ValueError("Mouth report differs from composition")
    if style.get("mouth_design") != design or c.get("mouth_design") != design:
        raise ValueError("Mouth style differs from composition")
    checks = {
        "mouth_half_width": "half_width",
        "mouth_corner_lift": "corner_lift",
        "mouth_seam_bow_height": "seam_bow",
    }
    if (
        any(c.get(k) != design[v] for k, v in checks.items())
        or c.get("mouth_seam_shoulder_radius_ratio") != 0.7
    ):
        raise ValueError("Mouth calibration differs from design")
    if (
        style.get("lip_pigment_width_m") != design["lip_band_width_m"]
        or style.get("lip_pigment_strength") != design["lip_strength"]
    ):
        raise ValueError("Mouth appearance differs from design")
    if report.get("facial_surface", {}).get("mouth_design") != record:
        raise ValueError("Mouth analytic record differs")
    bound = c["height_m"] * design["muzzle_projection"]
    if not math.isclose(
        record.get("authored_support_envelope_m", -1),
        bound,
        rel_tol=1e-12,
        abs_tol=1e-14,
    ):
        raise ValueError("Invalid mouth support envelope")
    motion = report.get("mouth_motion", {})
    if motion.get("target_names") != NAMES or motion.get("new_targets") != NAMES[3:]:
        raise ValueError("Missing actual mouth target names")
    if (
        motion.get("sampled_combinations") != 81
        or motion.get("blink_overlap_vertices") != 0
        or motion.get("outside_owned_components_unchanged") is not True
    ):
        raise ValueError("Mouth sampled support checks failed")
    for key in [
        "sampled_triangles",
        "minimum_sampled_area_m2",
        "minimum_sampled_normal_dot",
    ]:
        value = motion.get(key)
        if type(value) not in (int, float) or not math.isfinite(value) or value <= 0:
            raise ValueError("Invalid mouth pose evidence: " + key)
    for name in NAMES[3:]:
        count = motion.get("changed_vertices", {}).get(name)
        distance = motion.get("max_displacement_m", {}).get(name)
        if (
            type(count) is not int
            or count <= 0
            or type(distance) not in (int, float)
            or not math.isfinite(distance)
            or not 0 < distance < 0.02
        ):
            raise ValueError("Missing actual mouth target displacement")
    return True


def validate_glb(raw, report):
    from mouth_export_binding import validate_export

    return validate_export(raw, report, NAMES)


def verify_mouth_artifacts(snapshot, report, root, prefix, generated):
    root = Path(root).resolve()
    artifacts = {}
    names = ["face_style.json"]
    if snapshot.get("mouth_design") is not None:
        names.append("facial_surface_preflight.json")
    for name in names:
        if snapshot.get("mouth_design") is None and prefix + name not in generated:
            continue
        path = (root / prefix / name).resolve()
        if (
            not path.is_relative_to(root)
            or not path.is_file()
            or hashlib.sha256(path.read_bytes()).hexdigest()
            != generated.get(prefix + name)
        ):
            raise ValueError("Missing or changed mouth artifact: " + name)
        artifacts[name] = json.loads(path.read_text())
    frozen = artifacts.get("facial_surface_preflight.json", {}).get(
        "mouth_library_sources", {}
    )
    if validate_report(snapshot, report, artifacts.get("face_style.json", {}), frozen):
        return validate_glb((root / prefix / "aster_head.glb").read_bytes(), report)
    return None
