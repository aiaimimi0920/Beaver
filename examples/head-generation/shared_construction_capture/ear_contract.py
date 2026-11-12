"""Private identity-only ear attachment prototype; no native/field admission.

Evaluated-shell identities and ear allocation ordinals must be bound by the
caller to actual native objects. The contract cannot infer identity from shape.
"""
from copy import deepcopy
from collections import Counter
from construction_topology import cycle, digest
from construction_weld_contract import exact_index, validate_weld
from construction_oral_contract import bind_oral_ring


def _ids(values):
    if (type(values) is not list or not values or any(type(p) is not str or not p for p in values)
            or len(set(values)) != len(values)):
        raise ValueError('Missing/duplicate typed POINT identity')
    return values


def _mesh(source):
    ids = _ids(source['point_ids'])
    if type(source['faces']) is not list or not source['faces']:
        raise ValueError('Actual source face incidence required')
    incidence, seen = {}, set()
    for index, face in enumerate(source['faces']):
        if type(face) is not list or len(face) < 3:
            raise ValueError('Malformed actual source face')
        for i in face:
            exact_index(i, len(ids), 'source face')
        if len(set(face)) != len(face):
            raise ValueError('Repeated source face POINT')
        key = min(cycle(face), cycle(list(reversed(face))))
        if key in seen:
            raise ValueError('Duplicate actual source face')
        seen.add(key)
        for a, b in zip(face, face[1:] + face[:1]):
            incidence.setdefault(tuple(sorted((a, b))), []).append((index, a, b))
    for rows in incidence.values():
        if len(rows) > 2 or len(rows) == 2 and rows[0][1:] == rows[1][1:]:
            raise ValueError('Nonmanifold/inconsistently oriented source incidence')
    return incidence


def _open_chain(chain, incidence):
    if len(chain) < 3 or len(set(chain)) != len(chain):
        raise ValueError('Attachment must be an ordered open chain')
    expected = {tuple(sorted(pair)) for pair in zip(chain, chain[1:])}
    declared = set(chain)
    induced = {edge for edge in incidence if set(edge) <= declared}
    if induced != expected or any(len(incidence[edge]) != 1 for edge in expected):
        raise ValueError('Attachment chain has a missing, closed, chord or nonboundary edge')
    for point in chain[1:-1]:
        boundary = {edge for edge, faces in incidence.items() if point in edge and len(faces) == 1}
        if boundary != {edge for edge in expected if point in edge}:
            raise ValueError('Attachment chain has an unreceipted boundary branch')
    return [incidence[tuple(sorted(pair))][0] for pair in zip(chain, chain[1:])]


def _seal(receipt):
    receipt.update(current_role_admission=False, field_transfer_admission=False, solver_ready=False)
    receipt['sha256'] = digest(receipt)
    return receipt


def bind_ear_attachment(evaluated_source, ear_source, *, side, root_source_indices,
                        ear_rows, lower_bridge, upper_bridge):
    """Bind actual root-row and BOTH neighboring bridge allocations by ordinal."""
    if type(side) is not int or side not in (-1, 1):
        raise ValueError('Explicit bilateral ear side required')
    shell_ids, ear_ids = _ids(evaluated_source['point_ids']), _ids(ear_source['point_ids'])
    if set(shell_ids) & set(ear_ids):
        raise ValueError('Shell and ear source identities overlap')
    shell_inc, ear_inc = _mesh(evaluated_source), _mesh(ear_source)
    if (type(root_source_indices) is not list or len(root_source_indices) < 5
            or type(ear_rows) is not list or len(ear_rows) != len(root_source_indices)):
        raise ValueError('Actual ordered root rows must cover at least five roots')
    for i in root_source_indices:
        exact_index(i, len(shell_ids), 'evaluated shell root')
    allocated = []
    for row in ear_rows:
        if type(row) is not list or not row:
            raise ValueError('Missing actual ear row allocation')
        allocated.extend(exact_index(i, len(ear_ids), 'ear row') for i in row)
    if len(allocated) != len(set(allocated)):
        raise ValueError('Repeated ear row allocation')
    for bridge in (lower_bridge, upper_bridge):
        if set(bridge) != {'source_index', 'ear_index'}:
            raise ValueError('Explicit bridge allocation pair required')
        exact_index(bridge['source_index'], len(shell_ids), 'evaluated bridge source')
        exact_index(bridge['ear_index'], len(ear_ids), 'ear bridge allocation')
        if bridge['ear_index'] in allocated:
            raise ValueError('Bridge allocation aliases an original root row')
    shell_chain = [lower_bridge['source_index'], *root_source_indices, upper_bridge['source_index']]
    ear_chain = [lower_bridge['ear_index'], *[row[0] for row in ear_rows], upper_bridge['ear_index']]
    shell_faces, ear_faces = _open_chain(shell_chain, shell_inc), _open_chain(ear_chain, ear_inc)
    mapped = dict(zip(ear_chain, shell_chain))
    for shell_face, ear_face in zip(shell_faces, ear_faces):
        if shell_face[1:] != (mapped[ear_face[2]], mapped[ear_face[1]]):
            raise ValueError('Shell/ear attachment face orientations do not oppose')
    pairs = [{'skin_point_id': shell_ids[s], 'ear_point_id': ear_ids[e],
              'evaluated_source_index': s, 'ear_index': e,
              'allocation_kind': 'lower_bridge' if i == 0 else 'upper_bridge' if i == len(shell_chain) - 1 else 'root_row',
              'root_row': i - 1 if 0 < i < len(shell_chain) - 1 else None}
             for i, (s, e) in enumerate(zip(shell_chain, ear_chain))]
    return _seal({'schema': 'private_ear_attachment_identity_v1', 'side': side,
        'evaluated_source': deepcopy(evaluated_source), 'ear_source': deepcopy(ear_source),
        'root_source_indices': list(root_source_indices), 'ear_rows': deepcopy(ear_rows),
        'lower_bridge': dict(lower_bridge), 'upper_bridge': dict(upper_bridge),
        'pairs': pairs, 'shell_edge_faces': shell_faces, 'ear_edge_faces': ear_faces,
        'scope': 'declared evaluated/allocation IDs; native evaluation and join binding still required'})


def _validate_ear(receipt):
    expected = bind_ear_attachment(receipt['evaluated_source'], receipt['ear_source'],
        **{k: receipt[k] for k in ('side', 'root_source_indices', 'ear_rows', 'lower_bridge', 'upper_bridge')})
    if expected != receipt:
        raise ValueError('Changed ear allocation/incidence receipt')


def compose_attachment_targets(oral_boundary, bilateral_ears, join_receipt, weld_receipt):
    """Admit only declared oral plus bilateral ear pair merges; no other merges."""
    if len(bilateral_ears) != 2 or {r['side'] for r in bilateral_ears} != {-1, 1}:
        raise ValueError('Exactly two explicitly sided ear receipts required')
    for ear in bilateral_ears:
        _validate_ear(ear)
    oral = bind_oral_ring(**{k: oral_boundary[k] for k in
        ('half_point_ids', 'seam_point_ids', 'skin_namespace', 'oral_point_ids', 'ring0_indices')})
    if oral != oral_boundary:
        raise ValueError('Changed declared oral boundary receipt')
    source = join_receipt['source']
    if digest(source) != join_receipt['source_sha256']:
        raise ValueError('Changed native join source')
    ids = _ids(source['points'])
    permutation = join_receipt['identity_maps']['POINT']
    if (any(type(i) is not int for i in permutation)
            or sorted(permutation) != list(range(1, len(ids) + 1))):
        raise ValueError('Incomplete actual join POINT permutation')
    before = weld_receipt['before']
    if (before['point_ids'] != [ids[i - 1] for i in permutation]
            or before['join_source_sha256'] != join_receipt['source_sha256']):
        raise ValueError('Join/weld point identity tables disagree')
    joined_faces = Counter(cycle(face['point_ids']) for face in source['faces'])
    before_faces = Counter(cycle([before['point_ids'][i] for i in face['points']]) for face in before['faces'])
    if joined_faces != before_faces:
        raise ValueError('Join/weld complete oriented face incidence disagrees')
    for ear in bilateral_ears:
        for mesh in (ear['evaluated_source'], ear['ear_source']):
            declared_faces = Counter(cycle([mesh['point_ids'][i] for i in face]) for face in mesh['faces'])
            if declared_faces - joined_faces:
                raise ValueError('Declared evaluated/ear face incidence absent from actual join')
    if weld_receipt.get('sha256') != digest({k: v for k, v in weld_receipt.items() if k != 'sha256'}):
        raise ValueError('Changed complete native weld receipt')
    native = validate_weld(before, weld_receipt['native_target_pairs'], weld_receipt['after_weld'])
    post = validate_weld(before, weld_receipt['native_target_pairs'], weld_receipt['after_mesh'])
    if native != weld_receipt['quotient'] or post != weld_receipt['mesh_quotient']:
        raise ValueError('Changed native/post-Mesh quotient')
    declarations = [{'component': 'oral', 'skin_point_id': r['skin_point_id'], 'component_point_id': r['oral_point_id']}
                    for r in oral['pairs']]
    for ear in bilateral_ears:
        declarations.extend({'component': 'ear:' + str(ear['side']), 'skin_point_id': r['skin_point_id'],
                             'component_point_id': r['ear_point_id']} for r in ear['pairs'])
        if not set(ear['ear_source']['point_ids']) <= set(ids) or not set(ear['evaluated_source']['point_ids']) <= set(ids):
            raise ValueError('Missing complete evaluated/ear source in actual join')
    if not set(oral['oral_point_ids']) <= set(ids):
        raise ValueError('Missing complete oral source in actual join')
    all_declared = [r[k] for r in declarations for k in ('skin_point_id', 'component_point_id')]
    if len(set(all_declared)) != len(all_declared) or not set(all_declared) <= set(ids):
        raise ValueError('Duplicate, crossed or missing attachment source identity')
    wanted = {frozenset((r['skin_point_id'], r['component_point_id'])): r for r in declarations}
    targets = {}
    for index, group in enumerate(post['point_sources']):
        contributors = frozenset(before['point_ids'][i] for i in group)
        if contributors in wanted and len(group) == 2:
            targets[contributors] = index
        elif len(group) != 1 or contributors & set(all_declared):
            raise ValueError('Unrelated/extra/missing attachment weld contributor')
    if set(targets) != set(wanted):
        raise ValueError('Incomplete declared native attachment weld coverage')
    rows = [{**r, 'target_index': targets[frozenset((r['skin_point_id'], r['component_point_id']))]} for r in declarations]
    post_inc = _mesh({'point_ids': ['post:' + str(i) for i in range(len(post['point_sources']))],
                      'faces': [f['points'] for f in weld_receipt['after_mesh']['faces']]})
    for component in ('oral', 'ear:-1', 'ear:1'):
        chain = [r['target_index'] for r in rows if r['component'] == component]
        pairs = list(zip(chain, chain[1:] + chain[:1])) if component == 'oral' else list(zip(chain, chain[1:]))
        if any(len(post_inc.get(tuple(sorted(pair)), [])) != 2 for pair in pairs):
            raise ValueError('Post-weld shared attachment edge lacks exactly two faces')
    return _seal({'schema': 'private_combined_attachment_targets_v1', 'rows': rows,
        'join_source_sha256': join_receipt['source_sha256'], 'native_quotient_sha256': native['sha256'],
        'post_mesh_quotient_sha256': post['sha256'], 'oral_boundary_sha256': oral['sha256'],
        'ear_receipt_sha256': [r['sha256'] for r in bilateral_ears],
        'scope': 'declared combined attachment identity only; native evaluation, fields and later surgery unadmitted'})
