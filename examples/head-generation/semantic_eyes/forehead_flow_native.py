"""Bind and mirror five forehead guides before the detached native transaction."""
import numpy as np
from forehead_flow_refine import refine_forehead_terminal
from forehead_flow_bind import bind_forehead_paths, mirror_forehead_paths
from forehead_flow_rule import propose_forehead_flow, forehead_chart, forehead_no_chart_crossings, forehead_open_boundary_guard


def prepare_forehead_flow(vertices, faces, regions, features, nasal_paths,
                          report, ordinals, calibrated_delta):
    old = np.asarray(vertices, float)
    binding = bind_forehead_paths(old, faces, regions, report)
    bases = [ordinals['Ear hollow pinna '+str(side)] for side in (-1, 1)]
    owners = {1: 1}
    for offset in (0, 100, 110):
        owners[bases[0]+offset] = bases[1]+offset
        owners[bases[1]+offset] = bases[0]+offset
    reflection = mirror_forehead_paths(old, faces, regions, binding, owners)
    mirror = reflection['point_mirror']
    protected = set(features['protected']) | set(binding['pins']) | set(reflection['negative_pins'])
    protected.update(i for p in nasal_paths for i in p)
    for key in ('nasal_spacing_source', 'nasal_rear_spacing_source',
                'orbital_support_source', 'mouth_corner_source'):
        stage = report[key]
        if not stage['transaction']['committed'] or not stage['transaction']['transfer_payload']['equal']:
            raise ValueError('Forehead refinement requires completed preceding transactions')
        protected.update(i for q in stage['affected_final_cycles'] for i in q)
    for q in report['chin_source_retile']['proposal']['replacement_faces']:
        protected.update(q)
    spec = propose_forehead_flow(old, faces, list(binding['positive_paths'].values()),
                                 protected, features['eye_height']+.018)
    spec = refine_forehead_terminal(old, faces, list(binding['positive_paths'].values()), spec)
    points = spec['vertices']
    lineage = spec['lineage']
    positive_faces = list(spec['affected_faces'])
    negative_faces = [reflection['face_mirror'][i] for i in positive_faces]
    for i, row in list(lineage.items()):
        partner = mirror[i]
        if partner == i or partner in lineage or partner in protected:
            raise ValueError('Forehead halves overlap a protected source')
        donors = [mirror[j] for j in row['donors']]
        lineage[partner] = {'donors': donors, 'weights': list(row['weights'])}
        points[partner] = np.asarray(np.asarray(row['weights']) @ old[donors], np.float32)
    affected = sorted(set(positive_faces) | set(negative_faces))
    if any(regions[i] != 1 for i in affected):
        raise ValueError('Forehead deformation crosses a non-skin owner')
    if set(lineage) & protected:
        raise ValueError('Forehead source moved a protected point')
    if not np.array_equal(points[sorted(protected)], old[sorted(protected)]):
        raise ValueError('Forehead source changed fixed functional geometry')
    for side_faces in (positive_faces, negative_faces):
        forehead_no_chart_crossings(forehead_chart(points), [list(faces[i]) for i in side_faces])
    old_blink = np.zeros_like(old, dtype=np.float32)
    new_blink = np.zeros_like(old, dtype=np.float32)
    used = {i for fi in affected for i in faces[fi]}
    for i in used:
        before, after = calibrated_delta(old[i]), calibrated_delta(points[i])
        if not np.isfinite([before, after]).all():
            raise ValueError('Nonfinite generated forehead blink field')
        old_blink[i, 2] = np.float32(old[i, 2]+before)-np.float32(old[i, 2])
        new_blink[i, 2] = np.float32(points[i, 2]+after)-np.float32(points[i, 2])
    spec.update(affected_faces=affected, roles=binding,
                additional_protected=protected, old_blink_delta=old_blink,
                new_blink_delta=new_blink,
                generated_blink_guard={'policy': 'Current calibrated position-derived closure; separately sampled both old and new fields'},
                guide_paths={'positive': binding['positive_paths'], 'negative': reflection['negative_paths']})
    spec['metrics']['bilateral_lineage_vertices'] = len(lineage)
    spec['metrics']['bilateral_moved_vertices'] = int(np.sum(np.any(points != old, axis=1)))
    spec['metrics']['cell_quality_statistics_scope'] = 'Positive half; mirrored half has the same per-cell statistics'
    spec['metrics']['ear_root_and_rim_pins_exact'] = True
    spec['metrics']['open_boundary_guard'] = forehead_open_boundary_guard(old, points, [list(q) for q in faces], affected)
    return spec
