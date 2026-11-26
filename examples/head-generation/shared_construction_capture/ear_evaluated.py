"""Private prototype: temporary markers observe evaluated Mirror, never apply it."""
from types import SimpleNamespace
from construction_topology import digest, stable_id
from construction_markers import attach, clear_verified, read, fields
from construction_mirror_contract import validate_mirror_lineage
from construction_native import _state_for, _verify_mesh_state, _decoded

EARS = {}


def modifier_contract(modifiers):
    """Reject every unproved evaluation branch before creating markers."""
    rows = []
    for m in modifiers:
        if m.type == 'SUBSURF' and m.levels == 0:
            rows.append({'type': m.type, 'name': m.name, 'levels': 0, 'render_levels': m.render_levels})
            continue
        if m.type != 'MIRROR':
            raise ValueError('Unsupported evaluated ear modifier')
        if (list(m.use_axis) != [True, False, False] or any(m.use_bisect_axis)
                or not m.use_mirror_merge or m.mirror_object is not None
                or not m.show_viewport):
            raise ValueError('Unsupported evaluated ear Mirror configuration')
        rows.append({key: getattr(m, key) for key in
                     ('type', 'name', 'merge_threshold', 'use_clip', 'show_viewport', 'show_render')})
    if sum(row['type'] == 'MIRROR' for row in rows) != 1:
        raise ValueError('Exactly one evaluated Mirror is required')
    return rows


def begin_evaluation(shell):
    import bpy
    if tuple(bpy.app.version) != (4, 3, 2):
        raise ValueError('Evaluated ear identity requires pinned Blender 4.3.2')
    state = _state_for(shell)
    if state is None or state['phase'] != 'half_created':
        raise ValueError('Evaluated ears require a bound unchanged shell half')
    _verify_mesh_state(shell, state)
    modifiers = modifier_contract(shell.modifiers)
    before = digest(fields(shell))
    source = state['source']
    evaluation = {'shell': shell, 'state': state, 'before': before,
                  'modifiers': modifiers, 'evaluated': None, 'closed': False, 'attached': False}
    try:
        attach(shell, {'POINT': list(range(1, len(source['point_ids']) + 1)),
                      'FACE': list(range(1, len(source['faces']) + 1)),
                      'CORNER': list(range(1, sum(len(f['corner_ids']) for f in source['faces']) + 1))})
        evaluation['attached'] = True
        bpy.context.view_layer.update()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        evaluated = shell.evaluated_get(depsgraph)
        evaluation['evaluated'] = evaluated
        data = evaluated.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)
        proxy = SimpleNamespace(data=data)
        observed = _decoded(proxy, source, read(proxy))
        receipt = validate_mirror_lineage(source, observed,
                                         seam_point_ids=state['packet']['seam_point_ids'], role_paths={})
        ids = [None] * len(data.vertices)
        for pid, sides in receipt['point_map'].items():
            for side, index in sides.items():
                ids[index] = stable_id(source['source_namespace'], 'mirrored_point',
                                       [pid, 'seam' if pid in receipt['seam_equivalence'] else side])
        if None in ids or len(ids) != len(set(ids)):
            raise ValueError('Incomplete evaluated point identities')
        evaluation.update(data=data, mirror=receipt,
                          source={'point_ids': ids, 'faces': [list(f.vertices) for f in data.polygons]})
        return evaluation
    except BaseException:
        close_evaluation(evaluation)
        raise


def close_evaluation(evaluation):
    if evaluation['closed']:
        raise ValueError('Evaluated ear lease already closed')
    evaluation['closed'] = True
    try:
        if evaluation['evaluated'] is not None:
            evaluation['evaluated'].to_mesh_clear()
    finally:
        shell = evaluation['shell']
        if evaluation['attached']:
            clear_verified(shell)
        if (digest(fields(shell)) != evaluation['before']
                or modifier_contract(shell.modifiers) != evaluation['modifiers']):
            raise ValueError('Read-only evaluated ear capture mutated source fields/modifiers')
        _verify_mesh_state(shell, evaluation['state'])


def bind_created_ear(obj, evaluation, side, roots, rows, lower_neighbors,
                     lower_id, upper_neighbors, upper_id, vertices, faces, lower_bridge_triangle):
    from ear_contract import bind_ear_attachment
    if evaluation['closed']:
        raise ValueError('Ear creation used retired evaluated identity lease')
    if len(lower_neighbors) != 1 or len(upper_neighbors) != 1:
        raise ValueError('Ear bridge source allocation is ambiguous')
    if len(obj.data.vertices) != len(vertices) or [list(f.vertices) for f in obj.data.polygons] != [list(f) for f in faces]:
        raise ValueError('Ear mesh helper changed producer allocation')
    if [list(v.co) for v in obj.data.vertices] != [list(v) for v in vertices]:
        # Blender storage rounds Python doubles; coordinates are not identity authority.
        import struct
        rounded = [[struct.unpack('f', struct.pack('f', c))[0] for c in v] for v in vertices]
        if [list(v.co) for v in obj.data.vertices] != rounded:
            raise ValueError('Ear mesh helper changed producer geometry')
    namespace = digest(['ear_producer', evaluation['state']['source']['source_namespace'], side])
    source = {'point_ids': [stable_id(namespace, 'point', [i]) for i in range(len(vertices))],
              'faces': [list(f) for f in faces]}
    receipt = bind_ear_attachment(evaluation['source'], source, side=side,
        root_source_indices=list(roots), ear_rows=[list(row) for row in rows],
        lower_bridge={'source_index': lower_neighbors[0], 'ear_index': lower_id},
        upper_bridge={'source_index': upper_neighbors[0], 'ear_index': upper_id})
    key = obj.as_pointer()
    if key in EARS:
        raise ValueError('Duplicate ear producer object')
    binding = seal_evaluation_binding(evaluation, source, receipt, lower_bridge_triangle)
    EARS[key] = {'object': obj, 'mesh': obj.data, 'name': obj.name,
                 'source': source, 'receipt': receipt, 'evaluation_mirror': evaluation['mirror'],
                 'ordinal_to_semantic_id': list(source['point_ids']), 'join_bound': False,
                 'evaluation_binding': binding}
    return receipt


def seal_evaluation_binding(evaluation, ear_source, receipt, lower_bridge_triangle):
    """Tie evaluated transport and producer allocation in a single immutable seal."""
    if receipt.get('sha256') != digest({k: v for k, v in receipt.items() if k != 'sha256'}):
        raise ValueError('Unsealed ear producer receipt')
    if receipt['evaluated_source'] != evaluation['source'] or receipt['ear_source'] != ear_source:
        raise ValueError('Receipt differs from actual evaluated/allocation source')
    triangle = list(lower_bridge_triangle)
    if (len(triangle) != 3 or len(set(triangle)) != 3
            or any(type(i) is not int or not 0 <= i < len(ear_source['point_ids']) for i in triangle)
            or sum(len(face) == 3 and set(face) == set(triangle) for face in ear_source['faces']) != 1):
        raise ValueError('Original lower bridge tuple is not one actual allocated triangle')
    binding = {'lower_bridge_triangle': triangle, 'schema': 'private_ear_evaluation_binding_v1',
               'shell_namespace': evaluation['state']['source']['source_namespace'],
               'evaluation_mirror_sha256': digest(evaluation['mirror']),
               'evaluated_source_sha256': digest(evaluation['source']),
               'ear_source_sha256': digest(ear_source),
               'ordinal_to_semantic_sha256': digest(ear_source['point_ids']),
               'ear_receipt_sha256': receipt['sha256']}
    binding['sha256'] = digest(binding)
    return binding
