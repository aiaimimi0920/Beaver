"""Bind explicit brow designs to emitted style, frozen source and native metrics."""

import ast
import hashlib
import json
import math

REQUIRED_SOURCES = {"brow_design.py", "brow_shapes.py", "brow_mesh.py", "brow_source.py"}


def positive(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def validate_brow_report(snapshot, report, style, frozen_sources):
    design = snapshot.get("brow_design")
    record = report.get("brow_design")
    if design is None:
        if record is not None or style.get("brow_design") is not None:
            raise ValueError("Unexpected brow design without composition binding")
        return None
    if not isinstance(record, dict) or record.get("parameters") != design or style.get("brow_design") != design:
        raise ValueError("Brow composition/report/emitted-style mismatch")
    expected = snapshot.get("brow_source_sha256")
    if not isinstance(expected, dict) or set(expected) != REQUIRED_SOURCES or record.get("source_sha256") != expected:
        raise ValueError("Missing or mismatched brow source binding")
    if not isinstance(frozen_sources, dict) or set(frozen_sources) != set(expected) or any(not isinstance(source, str) or hashlib.sha256(source.encode()).hexdigest() != expected[name] for name, source in frozen_sources.items()):
        raise ValueError("Emitted brow source differs from composition")
    sides = record.get("sides")
    if not isinstance(sides, list) or len(sides) != 2 or any(not isinstance(s, dict) or type(s.get("side")) is not int for s in sides) or {s.get("side") for s in sides} != {-1, 1}:
        raise ValueError("Brow evidence requires exactly two native sides")
    islands = design["detail_count"] if design["kind"] == "symbols" else 1
    requested = design["clearance_m"] - design["thickness_m"]
    for side in sides:
        if type(side["side"]) is not int or side.get("kind") != design["kind"] or side.get("islands") != islands or side.get("manifold_edges") is not True:
            raise ValueError("Invalid native brow identity/topology evidence")
        vertices, triangles = side.get("vertices"), side.get("triangles")
        if type(vertices) is not int or type(triangles) is not int or vertices < 6 * islands or triangles != 2 * vertices - 4 * islands:
            raise ValueError("Native closed brow topology counts inconsistent")
        areas = side.get("local_areas_m2")
        if not isinstance(areas, list) or len(areas) != islands or not all(positive(a) for a in areas) or not positive(side.get("signed_volume_m3")):
            raise ValueError("Invalid native brow area/volume")
        actual = side.get("sampled_min_depth_clearance_m")
        vertex = side.get("requested_vertex_depth_clearance_m")
        if not positive(actual) or actual < max(.00005, requested * .25) or not positive(vertex) or not math.isclose(vertex, requested, abs_tol=1e-10):
            raise ValueError("Missing or insufficient sampled brow clearance")
        if type(side.get("clearance_samples")) is not int or side["clearance_samples"] < vertices // 2:
            raise ValueError("Missing native brow clearance samples")
    return {"status": "native_brow_metrics_bound", "side_count": 2, "scope": "sampled geometry, not aesthetic or global intersection acceptance"}


def verify_brow_artifacts(snapshot, report, root, prefix, generated):
    artifacts = {}
    root = root.resolve()
    names = ("face_style.json", "semantic_bundle.py")
    legacy = snapshot.get("brow_design") is None
    if legacy:
        names = ("face_style.json",) if prefix + "face_style.json" in generated else ()
    for name in names:
        path = (root / prefix / name).resolve()
        if not path.is_relative_to(root) or not path.is_file() or generated.get(prefix + name) != hashlib.sha256(path.read_bytes()).hexdigest():
            raise ValueError("Missing or changed emitted brow artifact: " + name)
        artifacts[name] = path.read_text()
    if legacy:
        return validate_brow_report(snapshot, report, json.loads(artifacts.get("face_style.json", "{}")), {})
    nodes = [n.value for n in ast.parse(artifacts["semantic_bundle.py"]).body if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "RETAINED_BROW_DESIGN_SOURCES" for t in n.targets)]
    if len(nodes) != 1:
        raise ValueError("Missing unique frozen brow source record")
    return validate_brow_report(snapshot, report, json.loads(artifacts["face_style.json"]), ast.literal_eval(nodes[0]))
