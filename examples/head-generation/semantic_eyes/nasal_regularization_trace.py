"""Bounded raw13 cheek continuation from allocated core path and current anchor.
Not arbitrary topology inference: geometric band/return-plane guards fail closed.
Native coordinate input: X lateral, Y negative-front depth, Z height.
"""
from collections import defaultdict
import numpy as np
from nasal_source_domain import current_source_features

def trace_outer_flow(vertices, quads, regions, ordinals, core_path, anchor):
    v = np.asarray(vertices, float)
    features = current_source_features(v, quads, regions, ordinals)
    span = float(features['eye_height'] - features['nose_peak'][2])
    if not 0.01 < span < 0.06 or len(core_path) < 2:
        raise ValueError('Unsupported nasal frame/core path')
    adj = defaultdict(set)
    inc = defaultdict(list)
    for q, r in zip(quads, regions):
        if r != 1:
            continue
        if len(q) != 4:
            raise ValueError('Shared cheek flow expects a quad skin source')
        q = list(map(int, q))
        for i, x in enumerate(q):
            adj[x].update([q[i - 1], q[(i + 1) % 4]])
            inc[x].append(q)
    if anchor not in adj[core_path[0]]:
        raise ValueError('Anchor/core handoff is not an actual source edge')
    if any((b not in adj[a] for a, b in zip(core_path, core_path[1:]))):
        raise ValueError('Core path is not source-edge connected')
    side = np.sign(v[anchor, 0])
    eye_depth = -np.mean(np.array(features['eye_centers']), axis=0)[1]
    return_plane = eye_depth - 1.5 * span
    height_band = 0.15 * span
    chain = [int(core_path[0]), int(anchor)]
    events = []
    for _ in range(24):
        previous, current = chain[-2:]
        if -v[current, 1] <= return_plane:
            if len(chain) < 4:
                raise ValueError('Cheek return plane reached before a meaningful outer strip')
            return dict(full_path=chain[:0:-1] + list(core_path), outer_from_anchor=chain[1:], termination='first crossing of source-relative outer-cheek return plane', normalized_depth_plane=1.5, events=events)
        opposite = adj[current] - {previous} - set((x for q in inc[current] if previous in q for x in q))
        candidates = []
        for n in opposite:
            if n in chain or n in core_path or np.sign(v[n, 0]) != side:
                continue
            if abs(v[n, 2] - v[anchor, 2]) > height_band:
                continue
            if v[n, 1] <= v[current, 1] + 1e-07:
                continue
            a = v[current] - v[previous]
            b = v[n] - v[current]
            turn = float(np.degrees(np.arccos(np.clip(a @ b / np.linalg.norm(a) / np.linalg.norm(b), -1, 1))))
            candidates.append((turn, n))
        candidates.sort()
        if not candidates:
            raise ValueError('Outer path has no unique semantically valid continuation')
        if candidates[0][0] > 70:
            raise ValueError('Outer continuation turns too abruptly for this profile')
        if len(candidates) > 1 and candidates[1][0] - candidates[0][0] < 15:
            raise ValueError('Ambiguous outer continuation: tangent margin under15degrees')
        n = candidates[0][1]
        events.append(dict(vertex=current, valence=len(adj[current]), opposite_candidates=sorted(map(int, opposite)), valid_candidates=[dict(vertex=int(j), turn_deg=t) for t, j in candidates], chosen=int(n)))
        chain.append(int(n))
    raise ValueError('Outer cheek trace exceeded bounded local budget')
