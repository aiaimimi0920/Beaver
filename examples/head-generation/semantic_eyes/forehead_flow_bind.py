"""Pure CPU live-graph binding. No historical vertex or face IDs.

Raw13 feature-flow contract only. The native transaction separately certifies fields and generated poses.
"""
import numpy as np


def bind_forehead_paths(vertices, faces, regions, report, tolerance=1e-7):
    v = np.asarray(vertices, dtype=float)
    faces = [tuple(map(int, q)) for q in faces]
    regions = np.asarray(regions)
    if v.ndim != 2 or v.shape[1] != 3 or not np.isfinite(v).all():
        raise ValueError('Invalid live points')
    if len(regions) != len(faces):
        raise ValueError('Region count mismatch')
    if report['semantic_rows'][-6:] != [1.685, 1.701, 1.720, 1.740, 1.760, 1.780]:
        raise ValueError('Unsupported semantic station contract')
    if len(report['semantic_lateral_stations']) != 13:
        raise ValueError('Expected raw13 source chart')
    adj = {}; shared = {}; all_incident = {}
    for fi, q in enumerate(faces):
        for i in q:
            all_incident.setdefault(i, []).append(fi)
        if regions[fi] != 1:
            continue
        if len(q) != 4:
            raise ValueError('Skin source must be quads')
        for i in q:
            shared.setdefault(i, set()).update(q)
        for a, b in zip(q, q[1:] + q[:1]):
            adj.setdefault(a, set()).add(b)
            adj.setdefault(b, set()).add(a)
    def one(values, why):
        values = list(values)
        if len(values) != 1:
            raise ValueError(f'Ambiguous {why}: {values}')
        return int(values[0]) if isinstance(values[0], np.integer) else values[0]
    def opposite(previous, current):
        return one((i for i in adj[current] if i not in shared[previous]), 'quad opposite')
    def straight(path, count):
        for _ in range(count):
            path.append(opposite(path[-2], path[-1]))
    def seed_line(center, steps):
        first = one((i for i in adj[center] if v[i,0] > tolerance), 'positive transverse seed')
        path = [center, first]
        straight(path, steps - 1)
        return path
    def down_turn(path):
        continuation = opposite(path[-2], path[-1])
        candidates = set(adj[path[-1]]) - {path[-2], continuation}
        candidates = [i for i in candidates if v[i,2] < v[path[-1],2] - tolerance]
        path.append(one(candidates, 'downward transverse turn'))
    angle = np.arctan2(v[:,0], -v[:,1])
    def lateral_turn(path, upper_arc=False):
        # At the upper-lateral eye bend, use outward angular direction.
        candidates = [i for i in adj[path[-1]] if i != path[-2]
                      and angle[i] > angle[path[-1]] + tolerance
                      and (not upper_arc or v[i,2] > v[path[-1],2] + tolerance)
                      and (upper_arc or i in shared[path[-2]])]
        path.append(one(candidates, 'outward lateral turn'))
    axis = [i for i in adj if abs(v[i,0]) <= tolerance]
    axis.sort(key=lambda i: v[i,2], reverse=True)
    if len(axis) < 6:
        raise ValueError('Missing upper centerline')
    axis = axis[:6]
    offset = float(report['head_assembly_y_offset_m'])
    # Three highest interior stations are unwarped at center; validate frame.
    for i, height in zip(axis[:4], [1.780,1.760,1.740,1.720]):
        if abs(v[i,2] - (height + offset)) > tolerance:
            raise ValueError('Upper centerline not in current raw13 assembly frame')
    names = ['red','orange','green','blue','purple']
    paths = {name: seed_line(center, count) for name,center,count in
             zip(names,axis[1:],[14,14,14,12,11])}
    green = paths['green']
    reference = report['feature_flow_layout']['inner_aperture_reference']
    rim = [green[-1]]
    for height in (1.701, 1.685):
        row = report['semantic_rows'].index(height)
        point = np.asarray(reference[row*13+12], float).copy()
        point[2] += offset
        index = one(np.flatnonzero(np.linalg.norm(v-point, axis=1) <= tolerance), 'authored upper rim')
        if index not in adj[green[-1]]:
            raise ValueError('Upper-rim source path is not continuous')
        green.append(index); rim.append(index)
    roots = [x['attachment_positions'][-1] for x in report['feature_flow_ear_source']['upper_attachment_skin']]
    point = one((p for p in roots if p[0] > 0), 'positive ear upper root')
    root = one(np.flatnonzero(np.linalg.norm(v-point,axis=1) <= tolerance), 'ear root')
    split = one(adj[green[-1]] & adj[root], 'ear upper shared split')
    green.extend((split, root))
    # Stop at first source-column station entering the 1.570 chin transition.
    # Midpoint of adjacent semantic rows avoids relying on floating-point exact z.
    lower_handoff = (1.570 + 1.580)/2 + offset
    pole_evidence = []
    def descend_side(path, allow_original_shell_poles):
        down_turn(path)
        visited=set(path)
        while v[path[-1],2] > lower_handoff:
            previous,current=path[-2:]
            candidates=[i for i in adj[current] if i not in shared[previous]]
            if len(candidates)==1:
                nxt=candidates[0]
            elif allow_original_shell_poles and len(adj[current])==5 and len(candidates)==2:
                # Current raw13 outer-eye/nasal handoffs have an inward branch.
                # Preserve original shell-side rail, never follow inward support.
                ordered=sorted(candidates,key=lambda i:angle[i])
                if angle[ordered[-1]]-angle[ordered[0]] <= tolerance:
                    raise ValueError('Original shell pole lacks unique posterior branch')
                nxt=ordered[-1]
                pole_evidence.append({'point':current,'incoming':previous,
                    'posterior_shell':nxt,'rejected_inward_support':ordered[0]})
            else:
                raise ValueError('Unrecognized side-column pole; need source provenance')
            if nxt in visited or v[nxt,2]>=v[current,2]-tolerance:
                raise ValueError('Side-column continuation is not strictly descending')
            path.append(nxt);visited.add(nxt)
            if len(visited)>len(v):raise ValueError('Unbounded source walk')
        if not 1.560+offset < v[path[-1],2] < lower_handoff:
            raise ValueError('Side-column skipped its chin handoff')
    blue=paths['blue'];descend_side(blue,False)
    purple=paths['purple'];descend_side(purple,True)
    if len(pole_evidence)!=2:
        raise ValueError('Unsupported original-shell handoff pole count')
    # Preserve the existing raw13 lower transition: original col11 -> fan -> col10.
    # This is guide-only continuation, never an instruction to move accepted chin.
    purple.append(opposite(purple[-2],purple[-1]))
    fan=purple[-1]
    if len(adj[fan])!=3:
        raise ValueError('Missing lower original-shell transition fan')
    inner=one((i for i in adj[fan] if i!=purple[-2]
               and angle[i]<angle[fan]-tolerance and v[i,2]<v[fan,2]-tolerance),
              'lower transition toward anterior chin')
    purple.append(inner)
    # Only upper corridor may be considered for fairing; all continuation below
    # upper eye-frame source height is immutable as a guide-only downstream tail.
    frozen_continuation=sorted({i for name in ('blue','purple') for i in paths[name]
                                if v[i,2]<=1.673+offset})
    # Root comes from CURRENT generated ear provenance, coordinate-only binding.
    root_points = [x['attachment_positions'][-1]
                   for x in report['feature_flow_ear_source']['upper_attachment_skin']]
    root_point = one((p for p in root_points if p[0] > 0), 'positive ear upper root')
    root = one(np.flatnonzero(np.linalg.norm(v-root_point,axis=1) <= tolerance), 'ear root exact match')
    if green[-1] != root:
        raise ValueError('Green source chain did not reach authored upper ear root')
    # Original outline identity, not necessarily open after posterior return.
    reference = report['feature_flow_layout']['inner_aperture_reference']
    for name, height in [('red', 1.760), ('orange', 1.740)]:
        row = report['semantic_rows'].index(height)
        point = np.array(reference[row*13+12], dtype=float)
        point[2] += offset
        if np.linalg.norm(v[paths[name][-1]]-point) > tolerance:
            raise ValueError('Outer path no longer reaches authored source outline')
    used = set()
    for name,path in paths.items():
        if len(path) != len(set(path)) or used.intersection(path):
            raise ValueError('Five paths overlap or self-repeat')
        used.update(path)
        if any(b not in adj[a] for a,b in zip(path,path[1:])):
            raise ValueError('Non-skin path edge')
    star = {i for fi in all_incident[root] for i in faces[fi]}
    pins = set(frozen_continuation) | star | set(rim) | {p[0] for p in paths.values()} | {paths[n][-1] for n in names}
    return {'positive_paths': paths, 'pins': sorted(pins),
            'ear_root_star': sorted(star), 'fixed_upper_rim': rim,
            'green_curve_span': [9, 14], 'side_pole_evidence': pole_evidence,
            'frozen_side_continuation':frozen_continuation,
            'side_jaw_handoff_height':lower_handoff,
            'incident_faces': [fi for fi,q in enumerate(faces) if used.intersection(q)],
            'binding': 'Current raw13 centerline, quad opposite adjacency and current ear-root coordinates; no historical IDs'}


def mirror_forehead_paths(vertices, faces, regions, positive, owner_mirror=None, tolerance=1e-7):
    """Exact mirror clone, including affected face cycles and optional ear owners.

    Caller supplies CURRENT semantic owner pairs for ears. With default {1:1},
    this certifies only skin face cycles, never ear ownership. No closest-point
    fallback or historical ID convention. Returned IDs belong to current mesh.
    """
    v=np.asarray(vertices,float);faces=[tuple(map(int,q))for q in faces]
    owners={1:1} if owner_mirror is None else dict(owner_mirror)
    if any(owners.get(b)!=a for a,b in owners.items()):
        raise ValueError('Mirror ownership must be involutive')
    paths=positive['positive_paths'];owned=set(sum(paths.values(),[]))|set(positive['pins'])
    domain=[fi for fi,q in enumerate(faces)if owned.intersection(q) and int(regions[fi]) in owners]
    needed=owned|{i for fi in domain for i in faces[fi]}
    mapping={}
    for i in needed:
        target=v[i].copy();target[0]*=-1
        matches=np.flatnonzero(np.linalg.norm(v-target,axis=1)<=tolerance)
        if len(matches)!=1:
            raise ValueError('Mirror correspondence not uniquely coincident')
        mapping[i]=int(matches[0])
    def cycle(q):
        return min(q[k:]+q[:k] for k in range(len(q)))
    lookup={}
    for fi,q in enumerate(faces):
        key=cycle(q)
        if key in lookup:
            raise ValueError('Duplicate oriented source face')
        lookup[key]=fi
    face_mapping={}
    for fi in domain:
        q=tuple(mapping[i]for i in reversed(faces[fi]))
        partner=lookup.get(cycle(q))
        if partner is None or int(regions[partner])!=owners[int(regions[fi])]:
            raise ValueError('Mirrored source winding/connectivity/owner differs')
        face_mapping[fi]=partner
    return {'negative_paths':{n:[mapping[i]for i in p]for n,p in paths.items()},
            'negative_pins':sorted(mapping[i]for i in positive['pins']),
            'point_mirror':mapping,'face_mirror':face_mapping,
            'verified_owner_mirror':owners}
