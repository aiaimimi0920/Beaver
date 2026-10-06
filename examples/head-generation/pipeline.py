from pathlib import Path
import bpy, json, hashlib, struct, re, bmesh
from face import *
from shell import build_face
from mouth import store_mouth_deltas
from glb_merge import merge_face

ROOT = Path(beaver_input("project.godot")).parent
OUT = "assets/aster/head_recovery_88/"
BASE = "assets/aster/head_recovery_49/"
GUIDE = Path(beaver_input("authoring/head_generation_guide.md")).read_text()
assert "去发侧脸轮廓检查" in GUIDE


def object_hash(o):
    value = {
        "name": o.name,
        "matrix": [list(r) for r in o.matrix_world],
        "vertices": [list(v.co) for v in o.data.vertices],
        "faces": [list(p.vertices) for p in o.data.polygons],
        "materials": [m.name for m in o.data.materials],
    }
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


bpy.ops.wm.open_mainfile(
    filepath=str(ROOT / BASE / "aster_head_editable.blend"), load_ui=False, use_scripts=False
)
body = {
    o.name: object_hash(o)
    for o in bpy.data.objects
    if o.type == "MESH" and all(m.name == "Body" for m in o.data.materials)
}
assert body, "No frozen body found"
for o in list(bpy.data.objects):
    if o.type == "MESH" and any(m.name == "Face" for m in o.data.materials):
        bpy.data.objects.remove(o, do_unlink=True)
report = build_face()
report["style_parameters"] = STYLE
build_eyes()
for obj in PARTS:
    if obj.data.attributes.get("MouthOpenDelta") is None:
        store_mouth_deltas(obj)
shell = next(o for o in PARTS if o.name == "Face shell editable half")
zero_faces = sum(p.area < 1e-12 for p in shell.data.polygons)
front_count = shell.get("front_surface_face_count", len(shell.data.polygons))
reversed_faces = sum(p.normal.y >= -1e-8 for p in list(shell.data.polygons)[:front_count])
under_count = shell.get("underside_face_count", len(shell.data.polygons) - front_count)
bad_return = sum(
    p.normal.z >= 0 for p in list(shell.data.polygons)[front_count : front_count + under_count]
)
bad_side = sum(p.normal.x <= 0 for p in list(shell.data.polygons)[front_count + under_count :])
assert bad_side == 0, "Chin side closure must face outward"
assert bad_return == 0, "Chin underside winding must face downward"
if zero_faces or reversed_faces:
    print(
        "INVALID_SHELL",
        json.dumps(
            [
                {
                    "index": p.index,
                    "area": p.area,
                    "normal": list(p.normal),
                    "vertices": [list(shell.data.vertices[i].co) for i in p.vertices],
                }
                for p in shell.data.polygons
                if p.area < 1e-12 or p.normal.y >= -1e-8
            ]
        ),
    )
assert zero_faces == 0 and reversed_faces == 0, "Invalid facial shell winding/area"
assert all(math.isfinite(c) for v in shell.data.vertices for c in v.co)
report["shell_geometry_checks"] = {
    "zero_area_faces": zero_faces,
    "reversed_front_faces": reversed_faces,
    "reversed_chin_underside_faces": bad_return,
    "reversed_chin_side_faces": bad_side,
    "finite_vertices": True,
}
report["authoring_guide_sha256"] = hashlib.sha256(GUIDE.encode()).hexdigest()
normal_checks = {}
for obj in PARTS:
    if any(tag in obj.name for tag in ["skin lid", "lash ", "Eye sclera"]):
        weighted = sum(poly.normal.y * poly.area for poly in obj.data.polygons)
        normal_checks[obj.name] = weighted
        assert weighted < 0, "Reversed facial surface: " + obj.name
report["front_surface_normal_checks"] = normal_checks
from crease_rules import assign_creases, measure_crease_effect

new_crease_edges, crease_counts = assign_creases(shell, front_count, H, Y0, width, C)
sub = shell.modifiers.new("Semantic quad surface refinement", "SUBSURF")
sub.subdivision_type = "CATMULL_CLARK"
sub.levels = STYLE["surface_subdivision_levels"]
sub.render_levels = sub.levels
report["surface_subdivision_levels"] = sub.levels
report["facial_crease"] = {
    "edge_counts": crease_counts,
    "evaluated_effect": measure_crease_effect(shell, new_crease_edges),
}

from eye_socket import fit_pocket_clearance

report["pocket_skin_clearance"] = fit_pocket_clearance(shell)
report["pocket_rear_closure"] = [
    {
        "name": o.name,
        "planarity_before_m": o["rear_cap_planarity_before_m"],
        "planarity_after_m": o["rear_cap_planarity_after_m"],
        "rear_boundary_vertices": o["pocket_back_count"],
    }
    for o in PARTS
    if o.get("pocket_back_count")
]

from ears import fit_ear_roots

report["ear_root_fit"] = fit_ear_roots(shell)
from mouth import validate_oral_clearance

report["oral_clearance"] = validate_oral_clearance(shell)

report["face_atlas_layout"] = "spatial_skin_eye_ear_flat_swatches_v2"
assert body == {o.name: object_hash(o) for o in bpy.data.objects if o.name in body}
# Face/hair swatch atlases preserve semantic colors in the NPR shader UV1.
for role in ["Face"]:
    parts = [o for o in PARTS if o["npr_role"] == role]
    colors = []
    for o in parts:
        c = tuple(o.data.color_attributes["Color"].data[0].color)
        if c not in colors:
            colors.append(c)
    for o in parts:
        c = tuple(o.data.color_attributes["Color"].data[0].color)
        idx = colors.index(c)
        for loop in o.data.loops:
            v = o.data.vertices[loop.vertex_index].co
            # Keep a nonzero UV footprint for stable tangents inside each flat swatch.
            u = (idx + 0.5 + 0.30 * max(-1, min(1, v.x / 0.15))) / len(parts)
            vv = 0.1 + 0.8 * max(0, min(1, (v.z - 1.48) / 0.35))
            if role == "Face":
                u = 0.60 + 0.35 * (idx + 0.5 + 0.15 * max(-1, min(1, v.x / 0.15))) / len(colors)
                vv = 0.91 + 0.07 * max(0, min(1, (v.z - 1.48) / 0.35))
                if o.get("skin_detail", False):
                    sx, sy = faceuv(v.x, v.z)
                    u = 0.02 + 0.53 * sx
                    vv = 0.02 + 0.96 * sy
            if o.get("iris_detail", False):
                detail = o.data.uv_layers["DetailUV"].data[loop.index].uv
                u = 0.60 + 0.35 * detail.x
                vv = 0.05 + 0.35 * detail.y
            if o.get("ear_detail", False):
                detail = o.data.uv_layers["DetailUV"].data[loop.index].uv
                u = 0.60 + 0.35 * detail.x
                vv = 0.50 + 0.35 * detail.y
            if o.get("hair_cap_detail", False):
                detail = o.data.uv_layers["DetailUV"].data[loop.index].uv
                u = 0.55 + 0.43 * detail.x
                vv = 0.05 + 0.90 * detail.y
            o.data.uv_layers["UVMap"].data[loop.index].uv = (u, 1 - vv)
            if role == "Hair":
                o.data.uv_layers["UV2"].data[loop.index].uv = (u, 1 - vv)
    from textures import bake_base_atlas

    image = bake_base_atlas(role, parts, colors, OUT)
    mat = bpy.data.materials[role]
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image
    uvnode = nodes.new("ShaderNodeUVMap")
    uvnode.uv_map = "UVMap"
    mapping = nodes.new("ShaderNodeMapping")
    mapping.inputs["Scale"].default_value = (1, -1, 1)
    mapping.inputs["Location"].default_value = (0, 1, 0)
    mat.node_tree.links.new(uvnode.outputs["UV"], mapping.inputs["Vector"])
    mat.node_tree.links.new(mapping.outputs["Vector"], tex.inputs["Vector"])
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.8
from textures import bake_control_maps

bake_control_maps(OUT)
# Assemble the complete head 12 mm lower onto the frozen original neck.
for part in PARTS:
    for vertex in part.data.vertices:
        vertex.co.z -= 0.021
report["head_assembly_y_offset_m"] = -0.021
# Save editable source before export joining. Body fingerprint is unchanged.
report.update(
    body_objects=len(body),
    body_fingerprints=body,
    parts=[
        {
            "name": o.name,
            "vertices": len(o.data.vertices),
            "quads": sum(len(p.vertices) == 4 for p in o.data.polygons),
        }
        for o in PARTS
    ],
    visual_approved=False,
    sdf_recalibration="new geometric horizontal-normal threshold; light sweep visual check pending",
    full_showcase="pending",
)
bpy.ops.wm.save_as_mainfile(
    filepath=beaver_output(OUT + "aster_head_editable.blend"), check_existing=False
)
# Export only the two regenerated runtime roles, then append them to the original GLB.
bpy.ops.object.select_all(action="DESELECT")
groups = {role: [o for o in PARTS if o["npr_role"] == role] for role in ["Face"]}
for role in ["Face"]:
    members = groups[role]
    bpy.ops.object.select_all(action="DESELECT")
    for o in members:
        o.select_set(True)
        bpy.context.view_layer.objects.active = o
        for m in list(o.modifiers):
            bpy.ops.object.modifier_apply(modifier=m.name)
        if role == "Face":
            for loop in o.data.loops:
                v = o.data.vertices[loop.vertex_index].co
                o.data.uv_layers["UV2"].data[loop.index].uv = faceuv(v.x, v.z + 0.021)
    bpy.context.view_layer.objects.active = members[0]
    bpy.ops.object.join()
    members[0].name = role
    members[0].data.name = role
    if role == "Face":
        bm = bmesh.new()
        bm.from_mesh(members[0].data)
        bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=0.000001)
        bmesh.ops.dissolve_degenerate(bm, edges=list(bm.edges), dist=0.0000001)
        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")
        bm.to_mesh(members[0].data)
        bm.free()
        # Honest geometry-consistent normals; no unmatched analytic shading field.
        data = members[0].data
        data.update()
        normals = [Vector((0, 0, 0)) for _ in data.vertices]
        for poly in data.polygons:
            for index in poly.vertices:
                normals[index] += poly.normal * poly.area
        for normal in normals:
            normal.normalize()
        corner_normals = [Vector((0, 0, 0)) for _ in data.loops]
        for poly in data.polygons:
            for loop_index in poly.loop_indices:
                corner_normals[loop_index] = (
                    normals[data.loops[loop_index].vertex_index]
                    if poly.use_smooth
                    else poly.normal.copy()
                )
        data.normals_split_custom_set(corner_normals)
        report["flat_geometric_triangles"] = sum(not poly.use_smooth for poly in data.polygons)
        delta = members[0].data.attributes.get("MouthOpenDelta")
        assert delta is not None
        moves = [Vector(v.vector) for v in delta.data]
        members[0].shape_key_add(name="Basis")
        opened = members[0].shape_key_add(name="MouthOpen")
        for i, d in enumerate(moves):
            opened.data[i].co += d
        opened.value = 0.0
        report["mouth_moving_vertices"] = sum(d.length > 1e-6 for d in moves)
        report["mouth_max_delta_m"] = max(d.length for d in moves)
        bpy.ops.wm.save_as_mainfile(
            filepath=beaver_output(OUT + "aster_head_runtime_morph.blend"), check_existing=False
        )

    for o in list(bpy.context.selected_objects):
        o.select_set(False)
for o in bpy.data.objects:
    if o.type == "MESH" and o.name == "Face":
        o.select_set(True)
# Temporary generated asset lives inside this job output directory, not the project.
tmp = Path(beaver_output(OUT + "aster_head.glb")).parent / "runtime_new.glb"
bpy.ops.export_scene.gltf(
    filepath=str(tmp),
    export_format="GLB",
    use_selection=True,
    export_texcoords=True,
    export_normals=True,
    export_tangents=True,
    export_yup=True,
    export_morph=True,
    export_morph_normal=True,
)

base_raw = (ROOT / BASE / "aster_head.glb").read_bytes()
raw, merge_report = merge_face(base_raw, tmp.read_bytes())
Path(beaver_output(OUT + "aster_head.glb")).write_bytes(raw)
tmp.unlink()
report.update(merge_report)
report["source_glb_sha256"] = hashlib.sha256(base_raw).hexdigest()
report["new_glb_sha256"] = hashlib.sha256(raw).hexdigest()
for name in ["hair_base.png", "hair_ilm.png"]:
    Path(beaver_output(OUT + name)).write_bytes((ROOT / BASE / name).read_bytes())
definition = (ROOT / BASE / "aster_definition.tres").read_text()
definition = re.sub(r' uid="uid://[^"]*"', "", definition)
definition = definition.replace("res://" + BASE, "res://" + OUT)
Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)
Path(beaver_output(OUT + "generation_report.json")).write_text(json.dumps(report, indent=2))
print(
    json.dumps(
        {k: v for k, v in report.items() if k not in ["parts", "body_fingerprints"]}, indent=2
    )
)
