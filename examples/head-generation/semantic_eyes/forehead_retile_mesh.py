"""Current-source forehead retile, native transport and pose validation."""
import numpy as np
from forehead_author_surface import regularize_forehead_author_surface
from forehead_author_pose_guard import guard_forehead_author_poses
from forehead_current_recipe import prepare_current_forehead
from forehead_flow_acceptance import validate_forehead_flow_result
from forehead_retile_apply import apply_forehead_retile_data
from forehead_retile_shape import guard_forehead_retile_poses


def prepare_forehead_source_retile(vertices, faces, regions, ordinals, report, open_delta):
    """Input is the explicitly verified author surface, before retile allocation."""
    return prepare_current_forehead(vertices, faces, regions, ordinals, report, open_delta)


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
    calibration = context['calibration']
    blink = lambda point: closure_delta(point[0], point[2]-assembly_offset, 'skin', calibration)
    mouth = lambda point, name: motion_delta(point, calibration, name)
    author, author_change = regularize_forehead_author_surface(old, faces, regions, ordinals)
    author_change['pose_correspondence'] = guard_forehead_author_poses(
        old, author, faces, author_change['changed_vertices'], old_open, blink, mouth)
    author_change['changed_points'] = [
        {'vertex': i, 'original': old[i].tolist(), 'author': author[i].tolist()}
        for i in author_change['changed_vertices']]
    proposal = prepare_forehead_source_retile(author, faces, regions, ordinals, context['report'], old_open)
    flow_acceptance = validate_forehead_flow_result(old_open, proposal)
    # The caller owns a detached mesh transaction. Do not reset or obscure the
    # original baseline: both changes are reported and verified separately.
    for vertex, position in zip(data.vertices, author):
        vertex.co = position
    data.update()
    print('Forehead source retile:', proposal['metrics'], flush=True)
    transport = apply_forehead_retile_data(data, proposal)
    calibration = context['calibration']
    poses = guard_forehead_retile_poses(author, faces, proposal['vertices'], proposal['faces'],
        proposal['affected_old'], proposal['affected_new'], old_open, transport['new_open_delta'],
        polygon_diagonals,
        blink, mouth)
    poses['baseline'] = 'Explicit regularized author surface, not the original input geometry'
    return {'source_lineage': proposal['lineage'],
            'source_lineage_baseline': 'Apply author_shape_change.changed_points to original input first',
            'author_shape_change': author_change, 'flow_acceptance': flow_acceptance,
            'affected_old_cycles': [faces[i] for i in proposal['affected_old']],
            'affected_final_cycles': [proposal['faces'][i] for i in proposal['affected_new']],
            'guide_paths': proposal['guide_paths'],
            'metrics': proposal['metrics'], 'roles': proposal['topology']['roles'],
            'retile_operations': proposal['topology']['operations'],
            'edge_lineage': proposal['edge_lineage'],
            'native_fields': {key: value for key, value in transport.items()
                              if key not in ('old_open_delta', 'new_open_delta')},
            'sampled_shape_guard': poses,
            'scope': 'Explicit bounded author-shape correction, conforming quad retile and full guide fairing; '
                     'no reference coordinates or saved model identifiers',
            'acceptance': 'Pending actual saved export, source fields and visual verification'}
