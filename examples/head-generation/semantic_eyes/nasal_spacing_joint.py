"""Commit bridge/spacing changes only after detached source and field checks."""
import numpy as np
from nasal_mesh_snapshot import nasal_mesh_snapshot, assert_nasal_mesh_snapshot_equal
from nasal_spacing_mesh import stage_nasal_bridge_spacing
from forehead_retile_mesh import stage_forehead_retile


def refine_nasal_bridge_spacing(data, ordinals, inner, records, generated_return=None, baseline=None, orbital_calibration=None, assembly_offset=None, mouth_context=None, forehead_context=None):
    import bpy
    import bmesh
    if data.shape_keys is not None:
        basis = np.asarray([tuple(v.co) for v in data.shape_keys.reference_key.data])
        if not np.array_equal(basis, np.asarray([tuple(v.co) for v in data.vertices])):
            raise ValueError('Nasal spacing input positions must equal reference Basis')
    original = nasal_mesh_snapshot(data)
    staged = data.copy()
    if data.shape_keys is not None and staged.shape_keys == data.shape_keys:
        bpy.data.meshes.remove(staged)
        raise ValueError('Nasal spacing copy shares original shape-key storage')
    transfer = backup = None
    try:
        if forehead_context is not None:
            if any(value is not None for value in (generated_return, baseline, orbital_calibration, mouth_context)):
                raise ValueError('Forehead retile conflicts with another source stage')
            result = stage_forehead_retile(staged, ordinals, forehead_context, assembly_offset)
        else:
            result = stage_nasal_bridge_spacing(staged, ordinals, inner, records, generated_return, baseline,
                                                orbital_calibration, assembly_offset, mouth_context, None)
        assert_nasal_mesh_snapshot_equal(original, nasal_mesh_snapshot(data),
                                         'Original during nasal spacing staging')
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
                                                     'Final nasal spacing transfer')
        except Exception:
            backup.to_mesh(data)
            data.update()
            assert_nasal_mesh_snapshot_equal(original, nasal_mesh_snapshot(data),
                                             'Nasal spacing rollback')
            raise
        result['transaction'] = {'committed': True, 'transfer_payload': proof,
                                 'original_untouched_until_final_validation': True}
        key = ('forehead_five_flow_source' if forehead_context is not None else
               'mouth_corner_source' if mouth_context is not None else
               'orbital_support_source' if orbital_calibration is not None else
               'nasal_spacing_source' if generated_return is None else 'nasal_rear_spacing_source')
        return {key: result}
    finally:
        if backup is not None:
            backup.free()
        if transfer is not None:
            transfer.free()
        bpy.data.meshes.remove(staged)
