"""Native-only optional-component selection before atlas/normal/morph assembly."""

import bpy
from mouth import store_mouth_deltas

SUPPLEMENTAL_PREFIXES = (
    "Inner forked lash blade ",
    "Upper swept lash tuft ",
    "Attached outer lash wing ",
    "Outer lateral eye liner ",
    "Outer companion lash ",
    "Lower outer lash tuft ",
    "Embedded lateral child ",
)


def select_components(parts, parameters):
    removed = []
    for obj in list(parts):
        omit = (
            parameters["supplemental_lashes"] == 0
            and not parameters.get("layered_lashes", 0)
        ) and obj.name.startswith(SUPPLEMENTAL_PREFIXES)
        if parameters.get("short_upper_tuft", 0) and obj.name.startswith(
            "Upper swept lash tuft "
        ):
            omit = False
        omit = omit or (
            parameters.get("layered_lashes", 0)
            and not parameters.get("upper_lash_layer", 0)
            and obj.name.startswith("Inner forked lash blade ")
        )
        omit = omit or (
            parameters["lid_fold"] == 0
            and obj.name.startswith("Tapered upper eyelid fold ")
        )
        omit = omit or (
            parameters.get("simple_iris", 0) == 1
            and obj.name.startswith("Pupil separate accent ")
        )
        if omit:
            removed.append(obj.name)
            parts.remove(obj)
            bpy.data.objects.remove(obj, do_unlink=True)
    if parameters["supplemental_lashes"] == 0 and not parameters.get(
        "layered_lashes", 0
    ):
        assert not any(
            obj.name.startswith(SUPPLEMENTAL_PREFIXES)
            and not (
                parameters.get("short_upper_tuft", 0)
                and obj.name.startswith("Upper swept lash tuft ")
            )
            for obj in parts
        ), "Omitted supplemental part retained"
    # Evaluated lash reconstruction replaces mesh datablocks and may discard
    # their earlier delta attributes. Recreate only missing data before joining.
    for obj in parts:
        attr = obj.data.attributes.get("MouthOpenDelta")
        if attr is None:
            store_mouth_deltas(obj)
        else:
            assert attr.domain == "POINT" and len(attr.data) == len(obj.data.vertices)
    return {
        "removed_components": removed,
        "retained_lash_components": [
            o.name for o in parts if o.name.startswith(SUPPLEMENTAL_PREFIXES)
        ],
        "selection_stage": "after retained-part attachment checks; before atlas and normal regions",
        "preselection_checks_are_not_final_component_counts": True,
    }


def final_runtime_evidence(path):
    import hashlib
    from pathlib import Path
    from collections import Counter

    obj = bpy.data.objects.get("Face")
    assert obj is not None and obj.type == "MESH"
    attr = obj.data.attributes.get("NormalRegion")
    assert attr is not None and attr.domain == "FACE"
    counts = Counter(int(item.value) for item in attr.data)
    assert counts[47] > 0 and counts[48] > 0, "Missing preserved ear normal regions"
    return {
        "runtime_blend_sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest(),
        "final_normal_region_counts": dict(sorted(counts.items())),
        "ear_region_ids": [47, 48],
        "ear_region_face_counts": {47: counts[47], 48: counts[48]},
    }


def validate_retained_tuft_roots(parts):
    from mathutils.bvhtree import BVHTree

    result = {}
    for side in (-1, 1):
        upper = next(o for o in parts if o.name == "Upper lash " + str(side))
        tuft = next(o for o in parts if o.name == "Upper swept lash tuft " + str(side))
        tree = BVHTree.FromPolygons(
            [v.co.copy() for v in upper.data.vertices],
            [tuple(p.vertices) for p in upper.data.polygons],
        )
        root_x = min(abs(v.co.x) for v in tuft.data.vertices)
        roots = [v for v in tuft.data.vertices if abs(abs(v.co.x) - root_x) < 1e-7]
        distances = [tree.find_nearest(v.co)[3] for v in roots]
        assert roots and all(
            d is not None and d < 0.0008 for d in distances
        ), "Retained short tuft root disconnected"
        result[str(side)] = {
            "retained_child": tuft.name,
            "sampled_root_vertices": len(roots),
            "maximum_root_distance_m": max(distances),
            "maximum_allowed_distance_m": 0.0008,
        }
    return result
