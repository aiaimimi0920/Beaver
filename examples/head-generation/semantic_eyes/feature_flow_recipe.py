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
    'source_quad_export',
    'source_skin_crossings',
    'source_rear_quad_guard',
    'source_ear_flow',
    'source_return_flow',
    'source_ear_validation',
    'source_feature_stage_policy',
    'source_weld_ownership',
)
DEPENDENCIES = (
    'source_ear_quad_solver',
    'ear_final_mapping',
    'ear_final_mesh',
    'ear_return_cap_solver',
    'ear_final_caps',
    'ear_flow_geometry',
    'ear_front_refinement',
    'lip_joint_solver',
    'lip_depth_solver',
    'ear_flow_mesh',
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
