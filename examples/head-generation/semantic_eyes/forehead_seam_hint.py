"""RECONSTRUCTED optional hint revalidation; no permanent model protection."""
import copy
import hashlib
import json
import numpy as np
from forehead_seam_support import forehead_seam_inputs,forehead_seam_field_sample,forehead_seam_integer_vector


def forehead_revalidate_seam_hint(previous_selection,coordinates_mm,variable_ids,
        frozen_field,support_ids,guide_paths,protected_ids,sample_field_differential,
        *,source_identity,previous_source_identity,budget_mm=.499,
        active_slack_mm=.002,max_holds=1):
    """Match vertex IDs to CURRENT columns, require exact coords/frozen baseline.

    Caller source identities must bind original author/chart/field provenance.
    Do not reuse across topology vertex-ID remaps absent explicit caller mapping.
    Returning no hint merely restores normal selection/free solving.
    """
    xy,ids,frozen,support,forbidden=forehead_seam_inputs(coordinates_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids)
    if not isinstance(source_identity,str) or not source_identity or not isinstance(previous_source_identity,str):raise ValueError('Explicit source provenance identity required')
    if not np.isfinite([budget_mm,active_slack_mm]).all() or min(budget_mm,active_slack_mm)<=0 or type(max_holds)is not int or not 0<=max_holds<=4:raise ValueError('Invalid hint policy')
    result={'held_vertices':[],'held_columns':[],'evidence':[],'hint_revalidated':True,'source_identity':source_identity,'rejected':[]}
    if source_identity!=previous_source_identity:
        result['rejected'].append({'reason':'Source provenance identity changed'});return result
    state=forehead_seam_field_sample(xy,frozen,sample_field_differential,budget_mm)
    if state is None:raise ValueError('Current accepted source invalid')
    index={int(v):i for i,v in enumerate(ids)};seen=set()
    for old in previous_selection.get('evidence',()):
        if len(result['held_vertices'])>=max_holds:break
        vertex=old.get('vertex')
        if isinstance(vertex,bool) or not isinstance(vertex,(int,np.integer)) or int(vertex)in seen:raise ValueError('Invalid/duplicate hint vertex')
        vertex=int(vertex);seen.add(vertex);reason=None
        if vertex not in index or vertex not in support or vertex in forbidden:reason='Vertex no longer eligible free nonguide support'
        else:
            i=index[vertex];prior=np.asarray(old.get('accepted_parameters_mm',()),float);baseline=np.asarray(old.get('frozen_field_reference_m',()),float)
            if prior.shape!=(2,) or baseline.shape!=(3,) or not np.isfinite(prior).all() or not np.isfinite(baseline).all():reason='Missing or malformed previous accepted/baseline evidence'
            elif not np.array_equal(xy[i],prior):reason='Accepted coordinates changed'
            elif not np.array_equal(frozen[i],baseline):reason='Frozen field baseline changed'
            elif not 0<=state[0][i]<=active_slack_mm:reason='Current field not feasible and active'
        if reason:result['rejected'].append({'vertex':vertex,'reason':reason});continue
        item=copy.deepcopy(old);item.update(columns=[2*i,2*i+1],parameter_index=i,accepted_field_slack_mm=float(state[0][i]),hint_revalidated=True)
        result['held_vertices'].append(vertex);result['held_columns'].extend(item['columns']);result['evidence'].append(item)
    return result


def forehead_seam_source_identity(arrays, faces):
    """Bind original source and immutable topology, not current trial geometry."""
    digest = hashlib.sha256()
    for value in arrays:
        array = np.ascontiguousarray(value, dtype=np.float64)
        if not np.isfinite(array).all():
            raise ValueError('Nonfinite immutable source identity')
        digest.update(str(array.shape).encode('ascii'))
        digest.update(array.tobytes())
    canonical_faces = [[int(i) for i in face] for face in faces]
    digest.update(json.dumps(canonical_faces, separators=(',', ':')).encode())
    return digest.hexdigest()


def forehead_next_stage_seam_hint(selection, parameters_mm, variable_ids,
        frozen_field, current_points, prior_points, source_identity):
    """Store the newly admitted scalar parameter representation, not a warm state.

    The previous solve proof is never changed. Exact world/baseline identity is
    required; next-stage matching remains exact despite round-trip ULP changes.
    """
    ids = forehead_seam_integer_vector(variable_ids, 'variable IDs')
    parameters = np.asarray(parameters_mm, float)
    frozen = np.asarray(frozen_field, float)
    points, prior = np.asarray(current_points, float), np.asarray(prior_points, float)
    if (parameters.shape != (len(ids), 2) or frozen.shape != (len(ids), 3)
            or points.ndim != 2 or points.shape[1] != 3 or prior.shape != points.shape
            or not all(np.isfinite(x).all() for x in (parameters, frozen, points, prior))
            or not isinstance(source_identity, str) or not source_identity
            or selection.get('source_identity') != source_identity):
        raise ValueError('Invalid admitted hint source/frame')
    index = {int(vertex): i for i, vertex in enumerate(ids)}
    selected = forehead_seam_integer_vector(selection.get('held_vertices', ()), 'held vertices', True)
    records = {int(row['vertex']): row for row in selection.get('evidence', ())}
    result = {'held_vertices': [], 'held_columns': [], 'evidence': [],
              'source_identity': source_identity, 'from_scalar_admission': True}
    for vertex in selected:
        vertex = int(vertex)
        if vertex not in index or vertex not in records or not 0 <= vertex < len(points):
            raise ValueError('Missing admitted held-point ownership')
        i = index[vertex]
        if not np.array_equal(points[vertex], prior[vertex]):
            raise ValueError('Held world coordinate changed')
        if not np.array_equal(np.asarray(records[vertex].get('frozen_field_reference_m')), frozen[i]):
            raise ValueError('Held frozen field baseline changed')
        result['held_vertices'].append(vertex)
        result['held_columns'].extend((2*i, 2*i+1))
        result['evidence'].append({'vertex': vertex, 'columns': [2*i, 2*i+1],
            'accepted_parameters_mm': parameters[i].tolist(),
            'frozen_field_reference_m': frozen[i].tolist(),
            'previous_stage_accepted_parameters_mm': copy.deepcopy(records[vertex]['accepted_parameters_mm']),
            'held_world_exact_after_scalar_admission': True})
    return result
