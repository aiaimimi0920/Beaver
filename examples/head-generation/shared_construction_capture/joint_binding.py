"""Private semantic upper-ear footprint; no coordinates, Blender, or global indices.
The caller supplies a producer-qualified ledger at one verified stage. A seal must
come from the reviewed source invocation, never from a candidate being admitted.
"""
import hashlib
import json


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     allow_nan=False).encode()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def edge(a, b):
    return tuple(sorted((a, b)))


def cycles(roles, reverse):
    """Exact operands of flow_reconnect_skin and upper cap allocation."""
    r = roles
    skin = [('I','J','middle','left'), ('middle','R','n','left'),
            ('n','L','I','left'), ('J','K','right','middle'),
            ('middle','right','s','R'), ('right','K','U','s')]
    cap = [('ear37','ear36','ear40','ear42'),
           ('ear40','R','n','ear42'), ('n','L','ear37','ear42')]
    rows = [('skin:'+str(i), [r[k] for k in row]) for i,row in enumerate(skin)]
    rows += [('cap:'+str(i), [r[k] for k in row]) for i,row in enumerate(cap)]
    # Skin orientation is an actual flow_ordered_quad operand; cap orientation
    # follows producer side in apply_final_ear, independently.
    return {slot: list(reversed(row)) if reverse[slot.split(':')[0]] else row
            for slot,row in rows}


def oriented_cycle(row):
    row = tuple(row)
    return min(row[i:]+row[:i] for i in range(len(row)))


def operands(original, generated, skin, side):
    """Translate live observer operands through the POINT ledger first."""
    require(side in (-1, 1), 'Invalid producer side')
    require(len(original) == 40 and len(set(original)) == 40,
            'Producer local order must contain forty unique POINT IDs')
    require(set(generated) == {40,41,42}, 'Upper cap birth contract changed')
    require(len(set(generated.values()))==3 and not set(generated.values()) & set(original),
            'Generated birth aliases a source POINT')
    require(set(skin) == {'I','J','K','U','s','left','middle','right'},
            'Skin source operand contract changed')
    r = dict(skin, L=original[27], R=original[39], n=generated[41],
             ear36=original[36], ear37=original[37],
             ear40=generated[40], ear42=generated[42])
    require(len(set(r.values())) == len(r), 'Semantic operands alias')
    return r


def describe(invocation, ledger):
    """Bind nine allocated cells and all boundary incidence before mutation.
    Faces contain id, points, corners [{id, point, uv, categorical}], categorical.
    Edges contain points, categorical. All identifiers are producer/stage IDs.
    Face storage order is irrelevant; winding and per-face corner order are not.
    """
    require(invocation['schema'] == 'upper-ear-source-v2', 'Unknown source contract')
    from field_schema import inventory, fields
    domains=inventory(ledger['field_schema'],invocation)
    side = invocation['side']
    require(ledger['side'] == side, 'Wrong-side ledger')
    require(ledger['stage_id'] == invocation['stage_id'], 'Wrong ledger stage')
    require(set(invocation['source_sha256']) == {'upper_cap','skin_reconnect'},
            'Missing exact source hashes')
    require(all(isinstance(v,str) and len(v)==64 and all(c in '0123456789abcdef' for c in v)
                for v in invocation['source_sha256'].values()), 'Malformed source hash')
    roles = operands(invocation['original'], invocation['generated'],
                     invocation['skin'], side)
    require(invocation['reverse']['cap'] == (side < 0), 'Cap side winding mismatch')
    require(type(invocation['reverse']['skin']) is bool, 'Missing skin winding operand')
    expected = cycles(roles, invocation['reverse'])
    points = ledger['point_ids']
    require(len(points)==len(set(points)), 'Duplicate POINT identity')
    require(set(invocation['original']) | set(roles.values()) <= set(points),
            'Producer points missing from ledger')
    require(set(ledger['point_fields'])==set(points), 'Incomplete POINT field ledger')
    for values in ledger['point_fields'].values():
        fields(values,domains['point'],'POINT')
    faces = ledger['faces']; by_cycle = {}; face_ids = set(); corner_ids = set()
    incidence = {}
    for f in faces:
        require(f['id'] not in face_ids, 'Duplicate FACE identity')
        face_ids.add(f['id'])
        fields(f['categorical'],domains['face'],'FACE')
        ps = f['points']; cs = f['corners']
        require(len(ps)>=3 and len(set(ps))==len(ps), 'Invalid source face')
        require(set(ps)<=set(points), 'Face references unknown point')
        require([c['point'] for c in cs]==ps, 'CORNER order mismatch')
        for c in cs:
            require(c['id'] not in corner_ids, 'Duplicate CORNER identity')
            corner_ids.add(c['id'])
            fields(c['uv'],domains['uv'],'UV')
            fields(c['categorical'],domains['corner'],'CORNER')
            require(isinstance(c['uv'],dict) and isinstance(c['categorical'],dict),
                    'Missing CORNER boundary fields')
        by_cycle.setdefault(oriented_cycle(ps), []).append(f)
        for a,b in zip(ps,ps[1:]+ps[:1]):
            incidence.setdefault(edge(a,b), []).append(f)
    selected = {}
    for slot,ps in expected.items():
        matches = by_cycle.get(oriented_cycle(ps), [])
        require(len(matches)==1, 'Missing, duplicate, or reversed semantic cell '+slot)
        selected[slot] = matches[0]
    selected_ids = {f['id'] for f in selected.values()}
    require(len(selected_ids)==9, 'Semantic cells alias')
    edge_fields = {}
    for e in ledger['edges']:
        fields(e['categorical'],domains['edge'],'EDGE')
        key = edge(*e['points'])
        require(key not in edge_fields, 'Duplicate EDGE field binding')
        edge_fields[key] = e['categorical']
    patch_edges = set()
    for f in selected.values():
        ps=f['points']
        patch_edges.update(edge(a,b) for a,b in zip(ps,ps[1:]+ps[:1]))
    seam={edge(roles['L'],roles['n']),edge(roles['n'],roles['R'])}
    rim={edge(roles['R'],roles['s']),edge(roles['s'],roles['U']),
         edge(roles['ear36'],roles['ear40']),edge(roles['ear40'],roles['R'])}
    boundaries=[]
    for key in sorted(patch_edges):
        fs=sorted(incidence[key], key=lambda f:f['id'])
        require(len(fs)<=2, 'Nonmanifold footprint edge')
        directions=[]
        for f in fs:
            ps=f['points']
            directions.append(next((a,b) for a,b in zip(ps,ps[1:]+ps[:1]) if edge(a,b)==key))
        if len(directions)==2:
            require(directions[0]==directions[1][::-1], 'Shared edge winding mismatch')
        require(key in edge_fields, 'Missing EDGE categorical fields')
        inside=[f for f in fs if f['id'] in selected_ids]
        outside=[f for f in fs if f['id'] not in selected_ids]
        role='skin_ear_shared' if key in seam else ('free_rim' if key in rim else
             ('outside_interface' if outside else ('open_boundary' if len(fs)==1 else 'patch_internal')))
        if key in seam:
            require(len(inside)==2 and not outside, 'Broken skin-ear seam incidence')
        if key in rim:
            require(len(fs)==1, 'Free rim has unexpected external cell')
        endpoints=[]
        for f in fs:
            endpoints.append({'face':f['id'], 'categorical':f['categorical'],
                              'corners':[c for c in f['corners'] if c['point'] in key]})
        uv_sets=[{c['point']:c['uv'] for c in f['corners'] if c['point'] in key} for f in fs]
        boundaries.append({'points':list(key),'role':role,
                           'inside':[f['id'] for f in inside],
                           'outside':[f['id'] for f in outside],
                           'uv_seam':len(uv_sets)==2 and uv_sets[0]!=uv_sets[1],
                           'categorical':edge_fields[key], 'endpoint_sources':endpoints})
    return {'schema':'upper-ear-footprint-v2','invocation':invocation,
            'semantic_points':roles,'cells':selected,'edges':boundaries,
            'point_fields':{p:ledger['point_fields'][p] for p in sorted(set(invocation['original'])|set(roles.values()))},
            'field_schema':ledger['field_schema'],
            'identity_scope':'producer ledger at verified stage; not historical model correspondence',
            'geometry_acceptance':False,'author_surface_acceptance':False}


def seal(invocation, ledger):
    description=describe(invocation, ledger)
    return description, digest(description)


def bind(invocation, ledger, reviewed_sha256):
    description=describe(invocation, ledger)
    require(digest(description)==reviewed_sha256,
            'Reviewed footprint changed: source, identity, order, UV, or categorical boundary')
    return description


def invocation_from_observer(state, point_ledger, side, stage_id, source_sha256, field_schema_sha256):
    """Read-only adapter for the existing skin_patch callback locals.
    Call after AllocationObserver has registered the new lineage points, before
    deletion. Object keys must be live ledger keys; no index/coordinate fallback.
    """
    def identity(vertex):
        require(vertex in point_ledger, 'Live operand has no transported POINT identity')
        return point_ledger[vertex]
    original=[identity(v) for v in state['original']]
    generated={i:identity(state['generated'][i]) for i in (40,41,42)}
    skin={key:identity(state[key]) for key in
          ('I','J','K','U','s','left','middle','right')}
    result={'schema':'upper-ear-source-v2','side':side,'stage_id':stage_id,
            'source_sha256':dict(source_sha256),'field_schema_sha256':field_schema_sha256,'original':original,
            'generated':generated,'skin':skin,
            'reverse':{'skin':state['reverse'],'cap':side<0}}
    operands(original,generated,skin,side)
    require(identity(state['R'])==original[39] and identity(state['L'])==original[27]
            and identity(state['n'])==generated[41], 'Actual root operands changed')
    return result
