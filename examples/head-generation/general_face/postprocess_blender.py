"""Run only inside the native job, after fresh base generation has finished."""
from pathlib import Path
import hashlib
import json
import bpy
from mathutils import Matrix
from general_face_controls import (
    transform_point, mouth_delta, inverse_transpose_normal, measure_profile, jacobian,
)
from glb_merge import merge_face
from general_face_glb_affine import transport_glb, assert_blender_positions


def frozen_hashes():
    return {o.name: object_hash(o) for o in bpy.data.objects
            if o.type == "MESH" and not any(m and m.name == "Face" for m in o.data.materials)}


def transform_object(obj, proportions, pivot, require_runtime_morph=False):
    assert obj.matrix_world == Matrix.Identity(4), "Nonidentity object frame is unsupported"
    mesh = obj.data
    assert mesh.users == 1, "Shared mesh would modify an unintended object"
    keys = mesh.shape_keys
    delta = mesh.attributes.get("MouthOpenDelta")
    if require_runtime_morph:
        assert keys is not None, "Runtime Face requires Basis and MouthOpen"
        assert delta is not None, "Runtime Face requires MouthOpenDelta"
    if keys is not None:
        assert set(keys.key_blocks.keys()) == {"Basis", "MouthOpen"}
        assert delta is not None, "Shape keys require MouthOpenDelta"
    if delta is not None:
        assert delta.domain == "POINT", "MouthOpenDelta must be POINT-domain"
        assert len(delta.data) == len(mesh.vertices), "MouthOpenDelta vertex count mismatch"
    import math
    before = measure_profile(v.co for v in mesh.vertices)
    normals = [tuple(n.vector) for n in mesh.corner_normals] if mesh.has_custom_normals else None
    if normals is not None:
        assert len(normals) > 0 and all(math.isfinite(c) for n in normals for c in n)
        assert all(sum(c * c for c in n) > 0 for n in normals), "Zero custom normal"
    if all(v == 1.0 for v in proportions.values()):
        return {"before": before, "after": before, "identity": True,
                "measured_axis_ratios": {key: 1.0 for key, span in before.items() if span > 0},
                "custom_normals_finite": normals is not None,
                "mouth_delta_present": delta is not None}
    # Snapshot every block first. Basis can alias mesh coordinates, so never
    # transform a live coordinate a second time after touching the mesh.
    vertices = [tuple(v.co) for v in mesh.vertices]
    snapshots = [(key, [tuple(v.co) for v in key.data]) for key in keys.key_blocks] if keys else []
    for vertex, point in zip(mesh.vertices, vertices):
        vertex.co = transform_point(point, proportions, pivot)
    for key, points in snapshots:
        for vertex, point in zip(key.data, points):
            vertex.co = transform_point(point, proportions, pivot)
    # Some legacy replacement editable lashes never received this attribute.
    # Preserve that absence; only final runtime meshes must carry mouth motion.
    if delta is not None:
        for item in delta.data:
            item.vector = mouth_delta(tuple(item.vector), proportions)
    mesh.update()
    if normals:
        mesh.normals_split_custom_set([inverse_transpose_normal(n, proportions) for n in normals])
    assert all(poly.area > 1e-12 for poly in mesh.polygons)
    if normals is not None:
        assert all(math.isfinite(c) for n in mesh.corner_normals for c in n.vector)
    after = measure_profile(v.co for v in mesh.vertices)
    ratios = {}
    for key, span in before.items():
        if span > 0:
            ratios[key] = after[key] / span
            assert math.isclose(after[key], span * proportions[key], rel_tol=2e-5, abs_tol=2e-7), (key, ratios[key], proportions[key])
        else:
            assert after[key] == 0, "Collapsed input axis acquired extent"
    return {"before": before, "after": after, "measured_axis_ratios": ratios,
            "identity": False, "custom_normals_transformed": normals is not None,
            "custom_normals_finite": normals is not None,
            "mouth_delta_present": delta is not None}


def transported_clearance_bounds(report, proportions):
    """Transport inherited bounds without relabeling them final measurements."""
    minimum = min(proportions.values())
    entries = []
    pocket = report.get("pocket_skin_clearance", {})
    for key in ("minimum_sampled_distance_m", "requested_margin_m"):
        if key in pocket:
            entries.append({"source": "pocket_skin_clearance." + key,
                            "baseline_m": pocket[key],
                            "transported_lower_bound_m": minimum * pocket[key],
                            "kind": "sampled distance" if key.startswith("minimum") else "requested margin, not measured"})
    margin = report["style_parameters"]["accent_evaluated_clearance_m"]
    entries.append({"source": "style_parameters.accent_evaluated_clearance_m",
                    "baseline_m": margin, "transported_lower_bound_m": minimum * margin,
                    "kind": "requested margin, not a verified final minimum"})
    return {"entries": entries,
            "scope": "Conditional distance bound sigma_min * baseline for the same two uniformly transformed face sets and pose. Legacy samples predate final refinements; these are not certified final clearances. Bounds do not apply between transformed face and frozen body/hair."}


def apply_general_face(config, pivot, out, declared_paths):
    def output(name):
        path = out + name
        assert path in declared_paths, "Undeclared output: " + path
        return beaver_output(path)

    report_path = Path(output("generation_report.json"))
    report = json.loads(report_path.read_text())
    proportions = config["proportions"]
    frozen = frozen_hashes()
    runtime = bpy.data.objects.get("Face")
    assert runtime and runtime.type == "MESH"
    assert all(m and m.name == "Face" for m in runtime.data.materials)
    runtime_measure = transform_object(runtime, proportions, pivot, require_runtime_morph=True)
    assert set(runtime_measure["measured_axis_ratios"]) == {"width", "height", "depth"}
    assert frozen == frozen_hashes(), "Body or hair changed"
    bpy.ops.wm.save_as_mainfile(filepath=output("aster_head_runtime_morph.blend"), check_existing=False)
    bpy.ops.object.select_all(action="DESELECT")
    runtime.select_set(True)
    bpy.context.view_layer.objects.active = runtime
    tmp = Path(output("general_face_export.glb"))
    bpy.ops.export_scene.gltf(filepath=str(tmp), export_format="GLB", use_selection=True,
                             export_texcoords=True, export_normals=True, export_tangents=True,
                             export_yup=True, export_morph=True, export_morph_normal=True)
    base_raw = (ROOT / BASE / "aster_head.glb").read_bytes()
    # Start from this job's freshly generated Face, never accepted final assets.
    fresh_face = Path(output("general_base_export.glb")).read_bytes()
    transported = transport_glb(fresh_face, proportions, pivot)
    position_agreement = assert_blender_positions(transported, tmp.read_bytes())
    Path(output("general_face_transported.glb")).write_bytes(transported)
    raw, merge_report = merge_face(base_raw, transported)
    Path(output("aster_head.glb")).write_bytes(raw)
    bpy.ops.wm.open_mainfile(filepath=output("aster_head_editable.blend"), load_ui=False, use_scripts=False)
    frozen_editable = frozen_hashes()
    names = {part["name"] for part in report["parts"]}
    assert names and len(names) == len(report["parts"])
    face_names = {o.name for o in bpy.data.objects if o.type == "MESH"
                  and any(m and m.name == "Face" for m in o.data.materials)}
    assert names == face_names, "Unexpected editable Face selection"
    editable = {}
    for name in sorted(names):
        obj = bpy.data.objects[name]
        assert all(m and m.name == "Face" for m in obj.data.materials)
        editable[name] = transform_object(obj, proportions, pivot)
    assert frozen_editable == frozen_hashes(), "Editable body or hair changed"
    bpy.ops.wm.save_as_mainfile(filepath=output("aster_head_editable.blend"), check_existing=False)
    report.update(merge_report)
    report["new_glb_sha256"] = hashlib.sha256(raw).hexdigest()
    report["general_face"] = {
        "schema_version": 1, "intent": config, "pivot_blender_xyz": pivot,
        "delivery_attribute_transport": "float32 accessor transport from same-job fresh Face export; Blender export retained as diagnostic",
        "fresh_face_export_sha256": hashlib.sha256(fresh_face).hexdigest(),
        "transported_face_export_sha256": hashlib.sha256(transported).hexdigest(),
        "blender_position_agreement": position_agreement,
        "jacobian": jacobian(proportions), "runtime_measurements": runtime_measure,
        "minimum_singular_value": min(proportions.values()),
        "maximum_singular_value": max(proportions.values()),
        "transported_clearance_bounds": transported_clearance_bounds(report, proportions),
        "editable_measurements": editable, "frozen_nonface_hashes": frozen,
        "fresh_generation_inputs": ["head_recovery_49", "head_recovery_617"],
        "legacy_measurements_scope": "All other geometric and clearance measurements precede the affine transform and may precede final 617 restoration/refinement",
        "editable_geometry_scope": "Editable cage predates inherited 617 restoration; runtime_morph.blend is the authoritative editable final geometry. Exact cage/runtime parity is not claimed.",
        "mouth_pose_scope": "Same-pose injective affine transport creates no new face self-intersections, but does not establish collision freedom of the original poses. MouthOpen 0/.25/.5/1 requires validation.",
        "sdf_status": "requires separate proportion-aware fresh bake",
        "visual_approved": False,
    }
    report["visual_approved"] = False
    report_path.write_text(json.dumps(report, indent=2))
