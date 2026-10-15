"""Runtime-safe Face node affine: preserve imported morph streams before scaling."""
import copy
import math
try:
    from general_face_controls import validate, vector
    from general_face_glb_affine import read_glb, write_glb, require, face_primitives, transport_glb, assert_blender_positions
except ModuleNotFoundError:
    from controls import validate, vector
    from glb_affine import read_glb, write_glb, require, face_primitives, transport_glb, assert_blender_positions

RUNTIME_POLICY = "face_node_affine_imported_baseline_v1"


def affine_parameters(proportions, pivot_blender, runtime_policy):
    require(runtime_policy == RUNTIME_POLICY, "Explicit supported runtime affine policy is required")
    config, pivot = validate(proportions), vector(pivot_blender)
    scales = [config['width'], config['height'], config['depth']]
    center = (pivot[0], pivot[2], -pivot[1])
    translation = [c * (1 - s) for c, s in zip(center, scales)]
    return scales, translation


def require_identity_node(node):
    for key, identity in [('matrix', [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
                          ('translation', [0, 0, 0]), ('rotation', [0, 0, 0, 1]), ('scale', [1, 1, 1])]:
        require(key not in node or node[key] == identity, "Existing Face transform must be identity")
    require('skin' not in node and not node.get('children'), "Face cannot own skin or child transforms")


def set_node_affine(node, scales, translation):
    require_identity_node(node)
    # glTF matrix and TRS are mutually exclusive, even for an identity matrix.
    node.pop('matrix', None)
    node['scale'] = scales
    node['translation'] = translation


def transport_face_node(raw, proportions, pivot_blender, runtime_policy):
    scales, translation = affine_parameters(proportions, pivot_blender, runtime_policy)
    # Validate the fresh source streams without changing a single buffer byte.
    transport_glb(raw, {}, pivot_blender)
    doc, binary = read_glb(raw)
    face_primitives(doc)
    if scales == [1.0, 1.0, 1.0]:
        return raw
    set_node_affine(doc['nodes'][0], scales, translation)
    return write_glb(doc, binary)


def merge_runtime_face(base_raw, face_raw, merge_face, proportions, pivot_blender, runtime_policy):
    scales, translation = affine_parameters(proportions, pivot_blender, runtime_policy)
    face_doc, _ = read_glb(face_raw)
    require(len(face_doc.get('nodes', [])) == 1, "Expected one incoming Face node")
    incoming = face_doc['nodes'][0]
    require(incoming.get('scale', [1, 1, 1]) == scales
            and incoming.get('translation', [0, 0, 0]) == translation,
            "Incoming Face node does not match requested affine")
    check = copy.deepcopy(face_doc)
    check['nodes'][0].pop('scale', None)
    check['nodes'][0].pop('translation', None)
    face_primitives(check)
    original, _ = read_glb(base_raw)
    face_ids = [i for i, node in enumerate(original['nodes']) if node.get('name') == 'Face']
    require(len(face_ids) == 1, "Expected one original Face node")
    face_id = face_ids[0]
    face_mesh = original['nodes'][face_id]['mesh']
    require(sum(node.get('mesh') == face_mesh for node in original['nodes']) == 1,
            "Face mesh is shared with another node")
    require_identity_node(original['nodes'][face_id])
    merged, report = merge_face(base_raw, face_raw)
    if scales != [1.0, 1.0, 1.0]:
        doc, binary = read_glb(merged)
        set_node_affine(doc['nodes'][face_id], scales, translation)
        for i, node in enumerate(original['nodes']):
            if i != face_id:
                require(doc['nodes'][i] == node, "Non-Face node changed")
        merged = write_glb(doc, binary)
    report['runtime_affine_policy'] = runtime_policy
    report['face_node_scale_godot_xyz'] = scales
    report['face_node_translation_godot_xyz'] = translation
    report['face_mesh_stream_policy'] = "Unchanged same-job fresh Face streams; affine follows engine baseline morph interpolation"
    return merged, report


def assert_native_world_positions(fresh_face_raw, blender_raw, proportions, pivot_blender):
    # CPU-only temporary value for verification; never delivered as runtime data.
    # This preserves the strict paired neutral/open position comparison even
    # though runtime delivery now keeps local positions and moves the Face node.
    expected_world = transport_glb(fresh_face_raw, proportions, pivot_blender)
    return assert_blender_positions(expected_world, blender_raw)
