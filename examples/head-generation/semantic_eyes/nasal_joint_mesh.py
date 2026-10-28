"""Commit all nasal stages once, after final-domain and original-shape checks."""
import numpy as np
from nasal_source_mesh import replace_nasal_source
from nasal_regularization_mesh import regularize_nasal_source
from nasal_inner_mesh import rebuild_inner_nasal_source
from nasal_inner_rule import propose_inner_nasal
from nasal_joint_contract import nasal_oriented_cycle, validate_nasal_joint_domain
from nasal_inner_adaptation import adapt_inner_nasal_rails
from nasal_mesh_snapshot import nasal_mesh_snapshot, assert_nasal_mesh_snapshot_equal


def refine_nasal_source(data, ordinals, authored):
    import bpy
    import bmesh
    source_vertices = np.asarray([tuple(v.co) for v in data.vertices])
    if data.shape_keys is not None:
        basis = np.asarray([tuple(v.co) for v in data.shape_keys.reference_key.data])
        if not np.array_equal(source_vertices,basis):
            raise ValueError('Joint nasal input positions must equal reference Basis')
    source_faces = [tuple(p.vertices) for p in data.polygons]
    region = data.attributes.get('NormalRegion')
    if region is None or region.domain != 'FACE':
        raise ValueError('Joint nasal source has no face ownership')
    source_regions = [int(x.value) for x in region.data]
    original_snapshot = nasal_mesh_snapshot(data)
    staged = data.copy()
    if data.shape_keys is not None and staged.shape_keys == data.shape_keys:
        bpy.data.meshes.remove(staged)
        raise ValueError("Staged nasal mesh shares original shape-key storage")
    expected = {}
    adapt_required = False

    def proposal(vertices, faces, regions, current_ordinals, paths):
        spec, points, final_faces = propose_inner_nasal(vertices,faces,regions,current_ordinals,paths)
        if not adapt_required:
            spec["rail_adaptation"] = {"status":"not_required_original_patch_gate_passed",
                "shape_contract":"Existing actual-export contract; no counterfactual diagonal pass claimed"}
            return spec,points,final_faces
        return adapt_inner_nasal_rails(source_vertices,source_faces,source_regions,
                                      vertices,faces,regions,spec,points,final_faces)

    def preflight(vertices, faces, regions, paths, regularization):
        spec, points, final_faces = proposal(vertices,faces,regions,ordinals,paths)
        proof = validate_nasal_joint_domain(vertices,faces,regions,regularization,spec,points,final_faces)
        expected.update(vertices=np.asarray(points,dtype=np.float32).astype(float),
                        faces=sorted(map(nasal_oriented_cycle,final_faces)),
                        regularization=regularization,proof=proof)
        return proof

    def actual_proposal(vertices, faces, regions, current_ordinals, paths):
        spec, points, final_faces = proposal(vertices,faces,regions,current_ordinals,paths)
        if (not np.array_equal(np.asarray(points,dtype=np.float32).astype(float),expected['vertices']) or
                sorted(map(nasal_oriented_cycle,final_faces))!=expected['faces']):
            raise ValueError('Native joint proposal differs from final-domain preflight')
        # Face indices may change after BMesh deletion/appending. Oriented cycles
        # and unchanged source vertex positions, not stale indices, are binding.
        return spec,points,final_faces

    bm = backup = None
    try:
        replacement = replace_nasal_source(staged,ordinals,authored)
        adapt_required = replacement['parameter_adaptation']['status'] != 'unchanged_original_gate_passed'
        regularization = regularize_nasal_source(staged,ordinals,replacement['allocated_flow_records'],preflight)
        inner = rebuild_inner_nasal_source(staged,ordinals,regularization['source_paths'],actual_proposal)
        mapping = {int(k):int(v) for k,v in inner['source_point_mapping'].items()}
        actual = np.asarray([tuple(v.co) for v in staged.vertices])
        if len(actual)!=len(mapping) or any(not np.array_equal(actual[j],expected['vertices'][i]) for i,j in mapping.items()):
            raise ValueError('Stored joint nasal vertices differ after source compaction')
        wanted = sorted(nasal_oriented_cycle([mapping[i] for i in q]) for q in expected['faces'])
        if sorted(nasal_oriented_cycle(p.vertices) for p in staged.polygons)!=wanted:
            raise ValueError('Stored joint nasal face topology differs after compaction')
        assert_nasal_mesh_snapshot_equal(original_snapshot,nasal_mesh_snapshot(data),"Original during staged nasal edit")
        staged_snapshot = nasal_mesh_snapshot(staged)
        bm = bmesh.new()
        bm.from_mesh(staged)
        bm.normal_update()
        backup = bmesh.new()
        backup.from_mesh(data)
        try:
            bm.to_mesh(data)
            data.update()
            transfer_proof = assert_nasal_mesh_snapshot_equal(staged_snapshot,nasal_mesh_snapshot(data),"Final nasal transfer")
        except Exception:
            backup.to_mesh(data)
            data.update()
            assert_nasal_mesh_snapshot_equal(original_snapshot,nasal_mesh_snapshot(data),"Nasal rollback")
            raise
        return dict(nasal_five_flow_source=replacement,nasal_expanded_regularization=regularization,
                    nasal_inner_source=inner,nasal_joint_transaction=dict(
                        committed=True,original_untouched_until_final_validation=True,
                        final_domain=expected['proof'],transfer_payload=transfer_proof,
                        runtime_acceptance='Pending actual automatic export, fields, poses and visual review'))
    finally:
        if backup is not None:
            backup.free()
        if bm is not None:
            bm.free()
        bpy.data.meshes.remove(staged)
