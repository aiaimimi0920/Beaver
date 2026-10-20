"""Bind researched nose design to source snapshots and emitted native evidence."""

import importlib.util
from nose_selection import LIBRARY

import hashlib
import json
import math

SOURCES = {"nose_design.py", "nose_field.py", "nose_mesh.py", "nose_source.py"}


def positive(value):
    return type(value) in (float, int) and math.isfinite(value) and value > 0


def validate_nose_report(snapshot, report, style, frozen):
    design = snapshot.get("nose_design")
    record = report.get("nose_design")
    if design is None:
        if (
            record is not None
            or style.get("nose_design") is not None
            or style.get("calibration", {}).get("nose_design") is not None
            or report.get("facial_surface", {}).get("nose_design") is not None
        ):
            raise ValueError("Unexpected researched nose without composition binding")
        return None
    if (
        not isinstance(record, dict)
        or record.get("parameters") != design
        or style.get("nose_design") != design
        or style.get("calibration", {}).get("nose_design") != design
    ):
        raise ValueError("Nose composition/report/style mismatch")
    expected = snapshot.get("nose_source_sha256")
    if (
        not isinstance(expected, dict)
        or set(expected) != SOURCES
        or record.get("source_sha256") != expected
    ):
        raise ValueError("Nose source identity missing or mismatched")
    if (
        not isinstance(frozen, dict)
        or set(frozen) != SOURCES
        or any(
            not isinstance(v, str)
            or hashlib.sha256(v.encode()).hexdigest() != expected[k]
            for k, v in frozen.items()
        )
    ):
        raise ValueError("Frozen nose source changed")
    analytic = report.get("facial_surface", {}).get("nose_design", {})
    if (
        analytic.get("parameters") != design
        or analytic.get("source_sha256") != expected
        or analytic.get("stations") != style["calibration"].get("nose_stations")
    ):
        raise ValueError("Nose analytic report/calibration mismatch")
    # Version 1 has a fixed analytic contract. Never execute embedded sources.
    module_spec = importlib.util.spec_from_file_location(
        "nose_evidence_design_v1", LIBRARY / "nose_design.py"
    )
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    calibration = style["calibration"]
    if calibration.get("nose_template_mode") != "rounded-quartic" or analytic.get(
        "stations"
    ) != module.stations(design):
        raise ValueError("Nose analytic stations or profile mode mismatch")
    height = calibration.get("height_m")
    envelope = analytic.get("authored_relief_upper_bound_m")
    if (
        not positive(height)
        or not positive(envelope)
        or not math.isclose(
            envelope,
            height * (design["projection"] + design.get("muzzle_projection", 0)),
            rel_tol=1e-12,
            abs_tol=1e-14,
        )
    ):
        raise ValueError("Missing analytic nasal envelope")
    meshes = record.get("attachments")
    count = 0 if design["kind"] == "fused" else 1
    if not isinstance(meshes, list) or len(meshes) != count:
        raise ValueError("Incorrect native nose attachment count")
    for m in meshes:
        if (
            not isinstance(m, dict)
            or m.get("kind") != design["kind"]
            or m.get("closed_two_manifold") is not True
            or not positive(m.get("signed_volume_m3"))
        ):
            raise ValueError("Invalid native nose topology or volume")
        v, f = m.get("vertices"), m.get("triangles")
        if type(v) is not int or type(f) is not int or v < 6 or f != 2 * v - 4:
            raise ValueError("Nose attachment topology counts inconsistent")
        if design["kind"] == "faceted-candidate" and m.get("flat_faces") != f:
            raise ValueError("Faceted module lost its flat construction")
    checks = report.get("nose_final_attachment_checks", [])
    if not isinstance(checks, list) or len(checks) != count:
        raise ValueError("Missing final skin attachment checks")
    for mesh, check in zip(meshes, checks):
        minimum, maximum = check.get("min_back_cap_embedding_m"), check.get(
            "max_back_cap_embedding_m"
        )
        if (
            check.get("name") != mesh["name"]
            or check.get("target_region") != 1
            or type(check.get("samples")) is not int
            or check["samples"] < mesh["vertices"] // 2
        ):
            raise ValueError("Nose final skin sample identity/count mismatch")
        if (
            type(minimum) not in (int, float)
            or not math.isfinite(minimum)
            or not positive(maximum)
            or minimum < -0.00001
            or not minimum <= maximum <= 0.008
        ):
            raise ValueError("Nose back cap not embedded in final sampled skin")
    for key in ("nose_refinement", "nose_arc_fairing"):
        post = report.get(key, {})
        if (
            post.get("selected_vertices") != 0
            or post.get("max_delta_m") != 0
            or post.get("outside_nose_unchanged") is not True
        ):
            raise ValueError("Unexpected legacy post-shaping on researched nose")
    return {
        "status": "native_nose_metrics_bound",
        "attachments": count,
        "scope": "Analytic design and measured attachment topology; not global intersection, SDF attachment shading, fit or aesthetic acceptance",
    }


def verify_nose_artifacts(snapshot, report, root, prefix, generated):
    root = root.resolve()
    artifacts = {}
    names = ["face_style.json"]
    if snapshot.get("nose_design") is not None:
        names.append("facial_surface_preflight.json")
    for name in names:
        path = (root / prefix / name).resolve()
        if snapshot.get("nose_design") is None and prefix + name not in generated:
            continue
        if (
            not path.is_relative_to(root)
            or not path.is_file()
            or generated.get(prefix + name)
            != hashlib.sha256(path.read_bytes()).hexdigest()
        ):
            raise ValueError("Missing or changed emitted nose artifact: " + name)
        artifacts[name] = json.loads(path.read_text())
    frozen = artifacts.get("facial_surface_preflight.json", {}).get(
        "nose_library_sources", {}
    )
    return validate_nose_report(
        snapshot, report, artifacts.get("face_style.json", {}), frozen
    )
