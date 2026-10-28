"""Versioned raw13 source-quad recipe; disabled unless explicitly requested."""
from importlib import import_module
from pathlib import Path

CONTRACT = 'raw13-feature-flow-v1'
ADAPTERS = (
    'source_density_flow',
    'source_chin_quad',
    'source_feature_flow',
    'source_mouth_flow',
    'source_chin_flow',
    'source_outer_flow',
    'source_orbital_continuity',
    'source_eye_ring_flow',
    'source_lip_relief_flow',
    'source_lip_joint_flow',
    'source_lip_depth_flow',
    'source_lip_seam_flow',
    'source_lip_patch_flow',
    'source_joint_frame_flow',
    'source_mouth_patch_sampling',
    'source_quad_export',
    'source_skin_crossings',
    'source_rear_quad_guard',
    'source_ear_flow',
    'source_return_flow',
    'source_ear_validation',
    'source_feature_stage_policy',
    'source_weld_ownership',
    'source_ear_weld_ownership',
    'source_nasal_flow',
    'source_chin_retile',
    'source_nasal_spacing',
)
DEPENDENCIES = (
    'source_ear_quad_solver',
    'ear_final_mapping',
    'ear_final_mesh',
    'ear_return_cap_solver',
    'ear_final_caps',
    'ear_lower_root_flow',
    'ear_flow_geometry',
    'ear_front_refinement',
    'lip_joint_solver',
    'lip_depth_solver',
    'lip_seam_shift',
    'lip_patch_solver',
    'mouth_patch_geometry',
    'mouth_patch_sampling',
    'ear_flow_mesh',
    'nasal_source_geometry',
    'nasal_source_template',
    'nasal_source_domain',
    'nasal_source_core',
    'nasal_source_binding',
    'nasal_source_boundary',
    'nasal_parameter_adaptation',
    'nasal_source_mesh',
    'nasal_regularization_chart',
    'nasal_regularization_trace',
    'nasal_regularization_solver',
    'nasal_joint_contract',
    'nasal_regularization_mesh',
    'nasal_inner_rule',
    'nasal_inner_adaptation',
    'nasal_inner_mesh',
    'nasal_mesh_snapshot',
    'nasal_joint_mesh',
    'chin_source_roles',
    'chin_source_fields',
    'chin_source_contract',
    'chin_source_template',
    'chin_source_solver',
    'chin_source_shape',
    'chin_source_mesh',
    'chin_joint_mesh',
    'nasal_spacing_rule',
    'nasal_spacing_shape',
    'nasal_spacing_mesh',
    'nasal_spacing_joint',
)



def adapt(sources, parameters):
    from sampling_contract import validate_feature_flow_contract
    validate_feature_flow_contract(parameters)
    out = dict(sources)
    for module_name in ADAPTERS:
        out = import_module(module_name).adapt(out)
    return out


def retained_sources():
    root = Path(__file__).resolve().parent
    names = ('feature_flow_recipe',) + ADAPTERS + DEPENDENCIES
    return {name + '.py': (root / (name + '.py')).read_text(encoding='utf-8') for name in names}
