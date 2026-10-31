"""Preserve proven shell selection membership at the authored skin/ear weld."""
import inspect


def ear_shell_boundary_edges(data):
    counts = {}
    for face in data.polygons:
        ids = list(face.vertices)
        for a, b in zip(ids, ids[1:] + ids[:1]):
            key = tuple(sorted((int(a), int(b))))
            counts[key] = counts.get(key, 0) + 1
    return {edge for edge, count in counts.items() if count == 1}


def validate_ear_shell_port_chain(vertices, edges):
    vertices = set(vertices)
    adjacency = {i: set() for i in vertices}
    for a, b in edges:
        if a not in adjacency or b not in adjacency or a == b:
            raise ValueError('Invalid shared ear boundary edge')
        adjacency[a].add(b)
        adjacency[b].add(a)
    if len(vertices) < 3 or sorted(map(len, adjacency.values())) != [1, 1] + [2] * (len(vertices) - 2):
        raise ValueError('Shared ear ports must form one nonbranching open chain')
    seen, pending = set(), [next(iter(vertices))]
    while pending:
        i = pending.pop()
        if i not in seen:
            seen.add(i)
            pending.extend(adjacency[i] - seen)
    if seen != vertices:
        raise ValueError('Disconnected shared ear boundary')


def capture_ear_shell_membership(members, target):
    from mathutils.kdtree import KDTree
    name = 'Continuous facial shell'
    shells = [obj for obj in members if obj.get('skin_detail') and obj.vertex_groups.get(name)]
    ears = [obj for obj in members if obj.get('ear_detail') and
            obj.name in ('Ear hollow pinna -1', 'Ear hollow pinna 1')]
    if len(shells) != 1 or len(ears) != 2 or len({obj.name for obj in ears}) != 2:
        raise ValueError('Unique authored shell and two named ears required')
    shell = shells[0]
    skin_regions = {int(row.value) for row in shell.data.attributes['NormalRegion'].data}
    if not skin_regions or not skin_regions <= {1, 2}:
        raise ValueError('Unexpected authored shell regions')
    transform = target.matrix_world.inverted()
    sx = transform @ shell.matrix_world
    skin_edges = ear_shell_boundary_edges(shell.data)
    skin_ports = {i for edge in skin_edges for i in edge}
    records, ear_rows = [], []
    for ear in sorted(ears, key=lambda obj: obj.name):
        ex = transform @ ear.matrix_world
        regions = {int(row.value) for row in ear.data.attributes['NormalRegion'].data}
        if not regions or regions & skin_regions:
            raise ValueError('Ear and shell source owners overlap')
        edges = ear_shell_boundary_edges(ear.data)
        ports = {i for edge in edges for i in edge}
        tree = KDTree(len(ear.data.vertices))
        for v in ear.data.vertices:
            tree.insert(ex @ v.co, v.index)
        tree.balance()
        matched, local = {}, []
        for i in sorted(skin_ports):
            v = shell.data.vertices[i]
            position = sx @ v.co
            hits = tree.find_range(position, 1e-6)
            if not hits:
                continue
            if len(hits) != 1 or hits[0][1] not in ports:
                raise ValueError('Ambiguous or non-boundary authored ear donor')
            j = int(hits[0][1])
            if j in matched.values():
                raise ValueError('Repeated authored ear boundary donor')
            groups = {shell.vertex_groups[g.group].name: float(g.weight) for g in v.groups}
            if groups.get(name) != 1.:
                raise ValueError('Authored shell donor lost its explicit membership')
            matched[i] = j
            local.append({'position': tuple(position), 'source_index': i, 'ear_index': j,
                          'ear_name': ear.name, 'ear_regions': sorted(regions), 'weight': groups[name]})
        common = {edge for edge in skin_edges if all(i in matched for i in edge)
                  and tuple(sorted(matched[i] for i in edge)) in edges}
        validate_ear_shell_port_chain(matched, common)
        records.extend(local)
        ear_rows.append({'name': ear.name, 'ports': len(local), 'shared_edges': len(common)})
    if len({row['source_index'] for row in records}) != len(records):
        raise ValueError('Authored shell donor shared by multiple ears')
    return {'group_name': name, 'skin_regions': sorted(skin_regions),
            'records': records, 'ears': ear_rows,
            'coordinate_frame': 'joined target Mesh local via captured object transforms'}


def restore_ear_shell_membership(obj, snapshot):
    from mathutils.kdtree import KDTree
    data = obj.data
    name = snapshot['group_name']
    group = obj.vertex_groups.get(name)
    if name != 'Continuous facial shell' or group is None:
        raise ValueError('Missing joined shell membership group')
    records = snapshot['records']
    skin = set(snapshot['skin_regions'])
    ears = {r for row in records for r in row['ear_regions']}
    membership = [set() for _ in data.vertices]
    region = data.attributes['NormalRegion']
    for face in data.polygons:
        for i in face.vertices:
            membership[i].add(int(region.data[face.index].value))
    targets = [i for i, owners in enumerate(membership) if owners & skin and owners & ears]
    if len(targets) != len(records):
        raise ValueError('Shared skin/ear weld cardinality changed')
    tree = KDTree(len(records))
    for i, row in enumerate(records):
        tree.insert(row['position'], i)
    tree.balance()
    before = [{obj.vertex_groups[g.group].name: float(g.weight) for g in v.groups} for v in data.vertices]
    positions = [tuple(v.co) for v in data.vertices]
    seen, pending = set(), []
    for i in targets:
        hits = tree.find_range(data.vertices[i].co, 1e-6)
        if len(hits) != 1 or hits[0][1] in seen:
            raise ValueError('Ambiguous post-weld ear donor correspondence')
        donor = int(hits[0][1]); row = records[donor]
        own = set(row['ear_regions'])
        if not membership[i] & own or membership[i] - (skin | own):
            raise ValueError('Shared ear vertex has unrelated source ownership')
        if row['weight'] != 1.:
            raise ValueError('Changed authored shell membership contract')
        seen.add(donor)
        pending.append((i, row['weight'], donor))
    if seen != set(range(len(records))):
        raise ValueError('Incomplete authored ear donor bijection')
    expected = [dict(row) for row in before]
    for i, weight, _ in pending:
        expected[i][name] = weight
    try:
        for i, weight, _ in pending:
            if before[i].get(name) != weight:
                group.add([i], weight, 'REPLACE')
        after = [{obj.vertex_groups[g.group].name: float(g.weight) for g in v.groups} for v in data.vertices]
        if after != expected or [tuple(v.co) for v in data.vertices] != positions:
            raise ValueError('Ear shell membership restoration altered unrelated payload')
    except Exception:
        for i, _, _ in pending:
            if name in before[i]:
                group.add([i], before[i][name], 'REPLACE')
            else:
                group.remove([i])
        raise
    return {'group_name': name, 'captured_ears': snapshot['ears'], 'boundary_vertices': len(pending),
            'changed': [{'target': i, 'position': positions[i], 'source_index': records[donor]['source_index'],
                         'ear_name': records[donor]['ear_name'], 'before': before[i].get(name), 'after': weight}
                        for i, weight, donor in pending if before[i].get(name) != weight],
            'geometry_changed': False, 'other_groups_changed': False,
            'policy': 'Unique authored shell donor owns only its named selection membership at shared ear ports'}


def adapt(sources):
    out = dict(sources)
    before = '    _oral_skin_donors = capture_oral_skin_donors(members,members[0])'
    after = '        report["oral_skin_point_ownership"] = restore_oral_skin_donors(members[0],_oral_skin_donors)'
    pipeline = out['pipeline.py']
    if pipeline.count(before) != 1 or pipeline.count(after) != 1:
        raise ValueError('Ear membership restoration requires existing oral weld stage')
    out['pipeline.py'] = pipeline.replace(before,
        '    _ear_shell_membership = capture_ear_shell_membership(members,members[0])\n' + before, 1).replace(after,
        after + '\n        report["ear_shell_membership"] = restore_ear_shell_membership(members[0],_ear_shell_membership)', 1)
    out['face.py'] += '\n\n' + '\n\n'.join(inspect.getsource(f) for f in
        (ear_shell_boundary_edges, validate_ear_shell_port_chain,
         capture_ear_shell_membership, restore_ear_shell_membership)) + '\n'
    return out
