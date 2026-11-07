"""Run with trusted Blender --background --factory-startup --disable-autoexec.
Only reads the input .blend; emits data-only absolute positions after strict checks.
"""

import sys, pathlib, json, hashlib, argparse
import bpy, numpy as np

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
from position_bridge import *
from native_snapshot import snapshot, values


def safety_state(obj):
    require(
        bpy.app.version == (4, 3, 2),
        "This strict baseline is validated only for Blender 4.3.2",
    )
    require(
        all(
            slot.link == "DATA" and slot.material == obj.data.materials[i]
            for i, slot in enumerate(obj.material_slots)
        ),
        "Object-linked material override introduced",
    )
    require(
        len(bpy.data.materials) == len(obj.data.materials),
        "Material data-block set changed",
    )
    require(
        len(bpy.data.objects) == 1 and len(bpy.data.meshes) == 1,
        "Unexpected objects or meshes",
    )
    require(len(bpy.data.libraries) == 0, "External Blender libraries are unsupported")
    require(len(bpy.data.texts) == 0, "Embedded text blocks are unsupported")
    require(len(bpy.data.actions) == 0, "Animation actions are unsupported")
    require(
        not obj.parent and len(obj.constraints) == 0,
        "Object parenting/constraints changed",
    )
    require(len(obj.modifiers) == 0, "Modifiers are unsupported")
    require(
        obj.type == "MESH" and obj.data.shape_keys is not None,
        "Expected mesh with shape keys",
    )
    require(
        obj.data.shape_keys.use_relative, "Absolute-timing shape keys are unsupported"
    )
    require(
        len(obj.data.shape_keys.key_blocks) == 8, "Expected Basis plus seven shape keys"
    )
    require(not bpy.data.armatures, "Armature/skinning introduced")
    for collection in [
        bpy.data.objects,
        bpy.data.meshes,
        bpy.data.shape_keys,
        bpy.data.materials,
        bpy.data.node_groups,
        bpy.data.scenes,
        bpy.data.worlds,
    ]:
        for block in collection:
            ad = getattr(block, "animation_data", None)
            require(
                not ad or (not ad.action and not ad.drivers and not ad.nla_tracks),
                "Animation/driver data introduced",
            )
    for mat in bpy.data.materials:
        ad = mat.node_tree.animation_data if mat.node_tree else None
        require(
            not ad or (not ad.action and not ad.drivers and not ad.nla_tracks),
            "Material animation introduced",
        )
    require(
        not bpy.context.preferences.filepaths.use_scripts_auto_execute,
        "Script autoexec must be disabled",
    )


def extract(blend, output, baseline_dir=None):
    require(
        not output.exists(), "Output directory already exists; no overwrite permitted"
    )
    before_sha = file_sha(blend)
    info, base, manifest = load_baseline(baseline_dir)
    bpy.context.preferences.filepaths.use_scripts_auto_execute = False
    bpy.ops.wm.open_mainfile(filepath=str(blend), load_ui=False, use_scripts=False)
    bpy.context.preferences.filepaths.use_scripts_auto_execute = False
    require(len(bpy.data.objects) == 1, "Object count changed")
    obj = next(iter(bpy.data.objects))
    safety_state(obj)
    if obj.mode != "OBJECT":
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.mode_set(mode="OBJECT")
    now, arrays = snapshot(obj, None)
    require(
        now["object_custom_properties"].get("V15_SOURCE_SHA256") == SOURCE_SHA,
        "Wrong source identity",
    )
    require(len(arrays["shape_0"]) == len(base["shape_0"]), "Vertex count changed")
    positions = np.stack([arrays["shape_" + str(i)] for i in range(8)])
    validate_positions(positions, info, base, manifest)
    # Blender Edit Mode may omit two implicit all-zero arrays. Canonicalize only
    # those known defaults, checking effective mesh properties before doing so.
    canonical_defaults = []
    for name, collection, prop, dtype in [
        ("sharp_edge", obj.data.edges, "use_edge_sharp", np.bool_),
        ("material_index", obj.data.polygons, "material_index", np.int32),
    ]:
        key = "attribute_" + name
        if key not in arrays:
            effective = values(collection, prop, dtype=dtype)
            require(
                not np.any(base[key]) and exact(effective, base[key]),
                "Missing non-default mesh attribute: " + name,
            )
            arrays[key] = effective
            now["attributes"].append(
                next(a for a in info["attributes"] if a["name"] == name)
            )
            canonical_defaults.append(name)
    # Only these exact Boolean selection schemas are transient UI state.
    selection_schema = {
        ".select_vert": ("POINT", len(obj.data.vertices)),
        ".select_edge": ("EDGE", len(obj.data.edges)),
        ".select_poly": ("FACE", len(obj.data.polygons)),
    }
    ignored_selection = {"attribute_" + name for name in selection_schema}
    for attribute in now["attributes"]:
        if attribute["name"] in selection_schema:
            domain, count = selection_schema[attribute["name"]]
            data = arrays["attribute_" + attribute["name"]]
            require(
                attribute["domain"] == domain
                and attribute["data_type"] == "BOOLEAN"
                and data.dtype == np.bool_
                and data.shape == (count,),
                "Selection attribute schema changed: " + attribute["name"],
            )

    def metadata_clean(x):
        x = dict(x)
        x.pop("arrays", None)
        x["attributes"] = sorted(
            [
                a
                for a in x["attributes"]
                if "attribute_" + a["name"] not in ignored_selection
            ],
            key=lambda a: a["name"],
        )
        return x

    require(
        metadata_clean(now) == metadata_clean(info),
        "Non-position native metadata changed",
    )
    direct_skip = (
        {"position", "attribute_position", "corner_normal", "vertex_normal"}
        | ignored_selection
        | {"shape_" + str(i) for i in range(8)}
    )
    require(
        set(arrays) - ignored_selection == set(base) - ignored_selection,
        "Native attribute set changed",
    )
    compared = []
    for key in base:
        if key not in direct_skip:
            require(
                exact(arrays[key], base[key]),
                "Non-position native array changed: " + key,
            )
            compared.append(key)
    # Native derived normals may vary with position. Reset only positions in this disposable
    # process to the pinned baseline, then check normals again to catch persistent normal edits.
    for i, key in enumerate(obj.data.shape_keys.key_blocks):
        key.data.foreach_set("co", base["shape_" + str(i)].ravel())
    obj.active_shape_key_index = 0
    obj.data.vertices.foreach_set("co", base["position"].ravel())
    obj.data.update()
    bpy.context.view_layer.update()
    normalized, _arr = snapshot(obj, None)
    for key in ["corner_normal", "vertex_normal"]:
        require(
            exact(_arr[key], base[key]),
            "Persistent native normal state changed after baseline-position reset: "
            + key,
        )
    require(file_sha(blend) == before_sha, "Input .blend changed during read")
    output.mkdir(parents=False, exist_ok=False)
    with (output / "positions.npz").open("xb") as f:
        np.savez_compressed(
            f,
            positions=positions,
            indices=arrays["triangles"],
            source_vertex_indices=arrays["attribute_V15_SOURCE_VERTEX_INDEX"],
        )
    report = {
        "format": "faye-v15-position-extraction-v1",
        "validation": "PASS",
        "blender_version": bpy.app.version_string,
        "source_sha256": SOURCE_SHA,
        "input_blend_sha256": before_sha,
        "baseline_pins": BASELINE_PINS,
        "shape_names": [x["name"] for x in info["shapes"]],
        "vertex_groups": now["groups"],
        "vertex_count": len(positions[0]),
        "triangle_count": len(arrays["triangles"]),
        "positions_npz_sha256": file_sha(output / "positions.npz"),
        "coordinate_system": "Blender local (x,-z,y) relative to glTF",
        "strict_non_position_arrays_compared": compared,
        "normal_guard": "Live derived normals excluded; native normals compared exactly after disposable in-memory baseline position reset.",
        "selection_ignored": sorted(ignored_selection),
        "implicit_default_attributes_canonicalized": canonical_defaults,
        "generic_attribute_order_ignored": True,
        "input_blend_unchanged": True,
        "embedded_code_executed": False,
        "autoexecute_disabled": True,
        "mesh_exporter_used": False,
        "shading_correctness_claim": False,
    }
    exclusive_json(output / "extraction.json", report)
    print("EXTRACTION_PASS", output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("blend", type=pathlib.Path)
    parser.add_argument("output_dir", type=pathlib.Path)
    parser.add_argument(
        "--baseline-dir",
        type=pathlib.Path,
        help="Private pinned baseline data; defaults to adjacent baseline/",
    )
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :])
    extract(args.blend.resolve(), args.output_dir.resolve(), args.baseline_dir)
