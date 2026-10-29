"""Current-source forehead retile, native transport and pose validation."""
import numpy as np
from nasal_source_domain import current_source_features
from forehead_retile_roles import prepare_forehead_star_topology
from forehead_retile_fair import fair_forehead_guides
from forehead_retile_support import refine_forehead_supports
from forehead_retile_mirror import mirror_forehead_retile
from forehead_retile_apply import apply_forehead_retile_data
from forehead_retile_shape import guard_forehead_retile_poses


def prepare_forehead_source_retile(vertices, faces, regions, ordinals, report, open_delta):
    features = current_source_features(vertices, faces, regions, ordinals)
    topology = prepare_forehead_star_topology(vertices, faces, regions, report, features)
    proposal = fair_forehead_guides(vertices, faces, regions, topology, features)
    proposal = refine_forehead_supports(vertices, open_delta, proposal)
    bases = [ordinals['Ear hollow pinna '+str(side)] for side in (-1, 1)]
    owners = {a+offset: b+offset for a, b in ((bases[0], bases[1]), (bases[1], bases[0]))
              for offset in (0, 100, 110)}
    return mirror_forehead_retile(vertices, faces, regions, proposal, owners, open_delta)


def stage_forehead_retile(data, ordinals, context, assembly_offset):
    from ear_surface import polygon_diagonals
    from semantic_eye_runtime import closure_delta
    from researched_mouth_runtime import motion_delta
    if assembly_offset != -.021 or data.shape_keys is not None:
        raise ValueError('Forehead source retile requires the current pre-morph assembly frame')
    region = data.attributes.get('NormalRegion')
    motion = data.attributes.get('MouthOpenDelta')
    if (region is None or region.domain != 'FACE' or region.data_type != 'INT'
            or motion is None or motion.domain != 'POINT' or motion.data_type != 'FLOAT_VECTOR'):
        raise ValueError('Forehead source ownership or motion attributes are missing')
    old = np.asarray([tuple(v.co) for v in data.vertices], dtype=np.float32)
    faces = [list(q.vertices) for q in data.polygons]
    regions = [int(row.value) for row in region.data]
    old_open = np.asarray([tuple(row.vector) for row in motion.data], dtype=np.float32)
    proposal = prepare_forehead_source_retile(old, faces, regions, ordinals, context['report'], old_open)
    print('Forehead source retile:', proposal['metrics'], flush=True)
    transport = apply_forehead_retile_data(data, proposal)
    calibration = context['calibration']
    poses = guard_forehead_retile_poses(old, faces, proposal['vertices'], proposal['faces'],
        proposal['affected_old'], proposal['affected_new'], old_open, transport['new_open_delta'],
        polygon_diagonals,
        lambda point: closure_delta(point[0], point[2]-assembly_offset, 'skin', calibration),
        lambda point, name: motion_delta(point, calibration, name))
    return {'source_lineage': proposal['lineage'],
            'affected_old_cycles': [faces[i] for i in proposal['affected_old']],
            'affected_final_cycles': [proposal['faces'][i] for i in proposal['affected_new']],
            'guide_paths': proposal['guide_paths'],
            'metrics': proposal['metrics'], 'roles': proposal['topology']['roles'],
            'retile_operations': proposal['topology']['operations'],
            'edge_lineage': proposal['edge_lineage'],
            'native_fields': {key: value for key, value in transport.items()
                              if key not in ('old_open_delta', 'new_open_delta')},
            'sampled_shape_guard': poses,
            'scope': 'Current-source conforming quad retile and full guide fairing; '
                     'no reference coordinates or saved model identifiers',
            'acceptance': 'Pending actual saved export, source fields and visual verification'}
