"""Identity-derived expected donor payload; legacy mutation is independently checked."""
from copy import deepcopy
from construction_markers import fields
from construction_topology import digest
from construction_oral_contract import compose_donor_targets

POINT_TYPES = {'FLOAT', 'INT', 'BOOLEAN', 'FLOAT_VECTOR', 'FLOAT2', 'FLOAT_COLOR', 'BYTE_COLOR'}
EDGE_FIELDS = {'crease_edge': 'FLOAT', 'FacialCreaseGroup': 'INT'}


def donor_schemas(member_fields):
    schemas = {}
    for state in member_fields:
        for name, attr in state['attributes'].items():
            if attr['domain'] != 'POINT' or name == 'position' or name.startswith('.'):
                continue
            if attr['type'] not in POINT_TYPES or name in schemas and schemas[name] != attr['type']:
                raise ValueError('Unsupported/conflicting oral source POINT schema')
            schemas[name] = attr['type']
    if schemas.get('Color') != 'FLOAT_COLOR':
        raise ValueError('Missing declared skin Color donor contract')
    return schemas


def zero(kind):
    if kind in ('FLOAT_COLOR', 'BYTE_COLOR'):
        return [0.0] * 4
    if kind in ('FLOAT_VECTOR', 'FLOAT2'):
        return [0.0] * (3 if kind == 'FLOAT_VECTOR' else 2)
    return False if kind == 'BOOLEAN' else 0.0 if kind == 'FLOAT' else 0


def incidence(state):
    edges = {}
    for index, face in enumerate(state['faces']):
        points = face[0]
        for a, b in zip(points, points[1:] + points[:1]):
            edges.setdefault(tuple(sorted((a, b))), []).append(index)
    return edges


def attribute(state, name, domain, kind):
    row = state['attributes'].get(name)
    if row is None or row['domain'] != domain or row['type'] != kind:
        raise ValueError('Oral donor attribute schema changed: ' + name)
    return row['values']


def expected_transfer(before, captured, targets):
    skin, oral = captured['skin_fields'], captured['oral_fields']
    skin_ids = captured['skin_point_ids']
    if len(skin_ids) != len(set(skin_ids)) or len(skin_ids) != len(skin['vertices']):
        raise ValueError('Incomplete original skin donor identity table')
    lookup = {pid: i for i, pid in enumerate(skin_ids)}
    expected = deepcopy(before)
    skin_groups = dict(skin['groups'])
    target_groups = {name: i for i, name in before['groups']}
    if len(target_groups) != len(before['groups']):
        raise ValueError('Ambiguous joined donor group names')
    rows = targets['rows']
    if len({row['target_index'] for row in rows}) != len(rows):
        raise ValueError('Duplicate oral target identity')
    for row in rows:
        source = lookup[row['skin_point_id']]
        target = row['target_index']
        if type(target) is not int or not 0 <= target < len(before['vertices']):
            raise ValueError('Unbound oral target ordinal')
        for name, kind in captured['point_schemas'].items():
            values = attribute(expected, name, 'POINT', kind)
            donor = skin['attributes'].get(name)
            if donor is not None:
                value = attribute(skin, name, 'POINT', kind)[source]
            else:
                value = zero(kind)
            values[target] = deepcopy(value)
        groups = {skin_groups[i]: weight for i, weight in skin['vertices'][source][3]}
        if groups.get('Continuous facial shell') != 1.0 or not set(groups) <= set(target_groups):
            raise ValueError('Original skin donor group authority missing')
        expected['vertices'][target][3] = sorted([[target_groups[name], value] for name, value in groups.items()])
    skin_inc, oral_inc, current_inc = incidence(skin), incidence(oral), incidence(before)
    skin_edges = {tuple(sorted(row[0])): i for i, row in enumerate(skin['edges'])}
    target_edges = {tuple(sorted(row[0])): i for i, row in enumerate(before['edges'])}
    if len(skin_edges) != len(skin['edges']) or len(target_edges) != len(before['edges']):
        raise ValueError('Duplicate donor edge topology')
    skin_regions = set(attribute(skin, 'NormalRegion', 'FACE', 'INT'))
    oral_regions = set(attribute(oral, 'NormalRegion', 'FACE', 'INT'))
    current_regions = attribute(before, 'NormalRegion', 'FACE', 'INT')
    if not skin_regions or not skin_regions <= {1, 2} or not oral_regions or skin_regions & oral_regions:
        raise ValueError('Declared source skin/oral owners conflict')
    edge_records = []
    for left, right in zip(rows, rows[1:] + rows[:1]):
        source = tuple(sorted((lookup[left['skin_point_id']], lookup[right['skin_point_id']])) )
        mouth = tuple(sorted((left['oral_index'], right['oral_index'])))
        target = tuple(sorted((left['target_index'], right['target_index'])))
        if len(skin_inc.get(source, [])) != 1 or len(oral_inc.get(mouth, [])) != 1:
            raise ValueError('Declared skin/oral edge is not a shared source boundary')
        adjacent = current_inc.get(target, [])
        owners = {current_regions[i] for i in adjacent}
        if len(adjacent) != 2 or not owners & skin_regions or not owners & oral_regions:
            raise ValueError('Actual oral target edge lost exact two-owner incidence')
        if source not in skin_edges or target not in target_edges:
            raise ValueError('Missing explicit source/target oral edge')
        for name, kind in EDGE_FIELDS.items():
            value = attribute(skin, name, 'EDGE', kind)[skin_edges[source]]
            attribute(expected, name, 'EDGE', kind)[target_edges[target]] = deepcopy(value)
        edge_records.append({'skin_edge': skin_edges[source], 'target_edge': target_edges[target]})
    return expected, edge_records


def verify_oral_transfer(obj, packets, legacy_snapshot, restore_function):
    if len(packets) != 1 or 'oral_join' not in packets[0] or 'native_weld' not in packets[0]:
        raise ValueError('Oral transfer requires actual constructor/join/weld receipts')
    packet = packets[0]
    weld = packet['native_weld']
    if weld.get('sha256') != digest({k: v for k, v in weld.items() if k != 'sha256'}):
        raise ValueError('Changed original native weld receipt')
    captured = packet['oral_join']
    if captured['sha256'] != digest({k: v for k, v in captured.items() if k != 'sha256'}):
        raise ValueError('Changed original oral donor capture')
    if captured['join_source_sha256'] != packet['native_join']['source_sha256']:
        raise ValueError('Original oral donors belong to a different join')
    targets = compose_donor_targets(captured['boundary'], packet['native_join'], packet['native_weld'])
    before = fields(obj)
    if digest(before) != digest(packet['native_weld']['after_native_fields']):
        raise ValueError('Unreceipted mutation between native weld and donor restoration')
    expected, edges = expected_transfer(before, captured, targets)
    report = restore_function(obj, legacy_snapshot)
    after = fields(obj)
    if digest(after) != digest(expected):
        error = ValueError('Legacy oral restoration differs from exact identity-derived donor policy')
        error.native_diagnostic = {'phase': 'oral_transfer', 'before': before, 'expected': expected, 'after': after}
        raise error
    receipt = {'scope': 'identity-verified original oral donor effect only; ear/fairing/later surgery unadmitted',
               'targets': targets, 'edges': edges, 'before': before, 'after': after,
               'expected_sha256': digest(expected), 'current_role_admission': False, 'solver_ready': False}
    receipt['sha256'] = digest(receipt)
    packet['oral_transfer'] = receipt
    packet['solver_ready'] = False
    return report
