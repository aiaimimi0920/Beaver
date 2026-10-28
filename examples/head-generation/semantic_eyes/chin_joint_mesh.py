"""One detached transaction for the shared lower-mouth source operation."""
import numpy as np
from nasal_mesh_snapshot import nasal_mesh_snapshot, assert_nasal_mesh_snapshot_equal
from chin_source_mesh import stage_chin_source_retile
from chin_source_solver import chin_retile_recipe, chin_propose_retile


def refine_chin_source(data, authored):
    import bpy
    import bmesh
    if data.shape_keys is not None:
        basis = np.asarray([tuple(v.co) for v in data.shape_keys.reference_key.data])
        current = np.asarray([tuple(v.co) for v in data.vertices])
        if not np.array_equal(current, basis):
            raise ValueError('Lower-mouth input positions must equal reference Basis')
    original = nasal_mesh_snapshot(data)
    staged = data.copy()
    if data.shape_keys is not None and staged.shape_keys == data.shape_keys:
        bpy.data.meshes.remove(staged)
        raise ValueError('Lower-mouth staged copy shares original shape-key storage')
    transfer = backup = None
    try:
        result = stage_chin_source_retile(staged, authored, chin_retile_recipe(),
                                         chin_propose_retile)
        assert_nasal_mesh_snapshot_equal(original, nasal_mesh_snapshot(data),
                                         'Original during lower-mouth staging')
        expected = nasal_mesh_snapshot(staged)
        transfer = bmesh.new()
        transfer.from_mesh(staged)
        transfer.normal_update()
        backup = bmesh.new()
        backup.from_mesh(data)
        try:
            transfer.to_mesh(data)
            data.update()
            proof = assert_nasal_mesh_snapshot_equal(expected, nasal_mesh_snapshot(data),
                                                     'Final lower-mouth transfer')
        except Exception:
            backup.to_mesh(data)
            data.update()
            assert_nasal_mesh_snapshot_equal(original, nasal_mesh_snapshot(data),
                                             'Lower-mouth rollback')
            raise
        result['transaction'] = {'committed': True, 'transfer_payload': proof,
                                 'original_untouched_until_final_validation': True}
        return {'chin_source_retile': result}
    finally:
        if backup is not None:
            backup.free()
        if transfer is not None:
            transfer.free()
        bpy.data.meshes.remove(staged)
