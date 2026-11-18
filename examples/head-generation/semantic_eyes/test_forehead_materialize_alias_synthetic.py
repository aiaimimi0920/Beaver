"""Corpus-free synthetic regression; no model IDs, files, native calls or baseline resets."""
import unittest
import copy
import numpy as np
from forehead_pattern_catalogue import forehead_pattern_catalogue, forehead_pattern_context, forehead_pattern_edges, forehead_pattern_edge, forehead_pattern_guide_corners
from forehead_pattern_materialize import forehead_pattern_materialize
from forehead_pattern_solver import forehead_pattern_solve

class ForeheadMaterializeAliasSyntheticTests(unittest.TestCase):

    def test_synthetic_invariants(self):

        def synthetic_case(permutation):
            old = np.array([[x, 0.0, y] for y in range(3) for x in range(3)], float)
            inverse = np.argsort(permutation)
            vertices = old[inverse].tolist()
            faces0 = [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]]
            faces = [[int(permutation[i]) for i in q] for q in faces0]
            path = [int(permutation[i]) for i in [1, 4, 7]]
            edges = {e for q in faces for e in forehead_pattern_edges(q)}
            parent = [{'edge': list(e), 'parent': list(e)} for e in sorted(edges)]
            top = {'vertices': copy.deepcopy(vertices), 'chart': copy.deepcopy(vertices), 'lineage': {}, 'face_owners': list(range(4)), 'edge_lineage': parent, 'faces': copy.deepcopy(faces), 'paths': {'role': path}, 'retiled_new_faces': [], 'retiled_old_faces': []}
            proposal = {'vertices': vertices, 'faces': faces, 'open_delta': [[0.0, 0.0, 0.0]] * 9, 'source_chart': vertices, 'lineage': {}, 'topology': top, 'paths': {'role': path}, 'protected': [], 'moving': [int(permutation[4])], 'affected_new': [], 'affected_old': []}
            proposal['topology']['vertices'] = proposal['vertices']
            proposal['topology']['chart'] = proposal['vertices']
            proposal['source_chart'] = proposal['vertices']
            nonneutral = forehead_pattern_edge(permutation[1], permutation[4])
            neutral = edges - {nonneutral}
            fields = {e: (0.5, 0, False, True) if e == nonneutral else (0.0, 0, False, True) for e in edges}
            discrete = {'semantic': [1] * 9}

            def sample(point):
                x, y = (point[0], point[2])
                ix = min(int(x), 1)
                iy = min(int(y), 1)
                u = x - ix
                v = y - iy
                donors = [int(permutation[i]) for i in [iy * 3 + ix, iy * 3 + ix + 1, (iy + 1) * 3 + ix + 1, (iy + 1) * 3 + ix]]
                weights = [(1 - u) * (1 - v), u * (1 - v), u * v, (1 - u) * v]
                return (point.tolist(), {'donors': donors, 'weights': weights})
            cat = forehead_pattern_catalogue(proposal, range(4), vertices, sample, discrete, neutral, {'role': (path[0], path[-1])})
            chosen = []
            for fi, q in enumerate(faces):
                mask = sum((1 << k for k, e in enumerate(forehead_pattern_edges(q)) if int(permutation[4]) in e))
                chosen.append(next((p for p in cat['patterns'] if p['face'] == fi and p['mask'] == mask and ('transition_rotation' not in p))))
            local = [path[0], cat['mids'][forehead_pattern_edge(path[0], path[1])], path[1], cat['mids'][forehead_pattern_edge(path[1], path[2])], path[2]]
            w = {'feasible': True, 'selected_base_faces': list(range(4)), 'patterns': chosen, 'points': cat['points'], 'lineage': cat['lineage'], 'new_midpoints': list(cat['mids'].values()), 'new_centers': list(cat['centers'].values()), 'guide_ends': cat['guide_ends'], 'paths': {'role': local}, 'forbidden_old': []}
            output, audit = forehead_pattern_materialize(proposal, w, vertices, proposal['open_delta'], discrete, fields, neutral)
            assert audit['new_points'] == 8 and audit['parent_chains'] == 1
            assert proposal['vertices'] == vertices and len(proposal['vertices']) == 9
            try:
                forehead_pattern_materialize(proposal, w, vertices, proposal['open_delta'], discrete, fields, neutral, birth_rows={0: w['lineage'][9]})
            except ValueError:
                pass
            else:
                raise AssertionError('Old birth reset accepted')
            bad = copy.deepcopy(w)
            bad['patterns'][0]['transition_rotation'] = {'removed': [cat['mids'][nonneutral], cat['centers'][0]], 'added': [0, 1]}
            try:
                forehead_pattern_materialize(proposal, bad, vertices, proposal['open_delta'], discrete, fields, neutral)
            except ValueError:
                pass
            else:
                raise AssertionError('Nonneutral bend accepted')

            def geometry(p):
                canonical = []
                for q in p['faces']:
                    coords = [tuple(p['vertices'][v]) for v in q]
                    canonical.append(min((tuple(coords[k:] + coords[:k]) for k in range(len(q)))))
                return (sorted(canonical), [p['vertices'][i] for i in p['paths']['role']])
            solved = forehead_pattern_solve(proposal, cat, time_limit=5.0, max_new_points=12, prescribed_routes={'role': local})
            assert solved['feasible']
            forehead_pattern_materialize(proposal, solved, vertices, proposal['open_delta'], discrete, fields, neutral)
            return geometry(output)
        reference = synthetic_case(np.arange(9))
        rng = np.random.default_rng(831)
        for _ in range(5):
            assert synthetic_case(rng.permutation(9)) == reference
        c = forehead_pattern_context({'x': [0, 1, 2, 3, 4, 5], 'other': [8, 9]}, {'x': (1, 4)})
        assert c['attachments'] == [('x', 1, 0, True), ('x', 4, 5, False)]
        assert (0, 1) in c['fixed_edges'] and (4, 5) in c['fixed_edges'] and ((8, 9) in c['fixed_edges'])
        v = [[x, 0.0, y] for y in range(4) for x in range(4)]
        f = [[y * 4 + x, y * 4 + x + 1, (y + 1) * 4 + x + 1, (y + 1) * 4 + x] for y in range(3) for x in range(3)]
        ee = {e for q in f for e in forehead_pattern_edges(q)}
        path = [1, 5, 9, 13]
        t = {'vertices': copy.deepcopy(v), 'chart': copy.deepcopy(v), 'lineage': {}, 'face_owners': list(range(9)), 'edge_lineage': [{'edge': list(e), 'parent': list(e)} for e in sorted(ee)], 'faces': f, 'paths': {'role': path}}
        p = {'vertices': v, 'faces': f, 'open_delta': [[0.0, 0.0, 0.0]] * 16, 'source_chart': v, 'lineage': {}, 'topology': t, 'paths': {'role': path}, 'protected': []}

        def sample_interior(point):
            x, y = (point[0], point[2])
            ix = min(int(x), 2)
            iy = min(int(y), 2)
            u = x - ix
            w = y - iy
            return (point.tolist(), {'donors': [iy * 4 + ix, iy * 4 + ix + 1, (iy + 1) * 4 + ix + 1, (iy + 1) * 4 + ix], 'weights': [(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w]})
        catalogue = forehead_pattern_catalogue(p, range(9), v, sample_interior, {'role': [1] * 16}, ee, {'role': (5, 9)})
        witness = forehead_pattern_solve(p, catalogue, time_limit=5, max_new_points=0, prescribed_routes={'role': [5, 9]})
        assert witness['feasible']
        out, a = forehead_pattern_materialize(p, witness, v, p['open_delta'], {'role': [1] * 16}, {e: (0.0, 0, False, True) for e in ee}, ee)
        assert out['paths']['role'] == path and a['new_points'] == 0
