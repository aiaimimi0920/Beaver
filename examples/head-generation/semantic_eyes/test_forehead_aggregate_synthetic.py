"""Corpus-free synthetic regression; no model IDs, files, native calls or baseline resets."""
import unittest
import copy
import numpy as np
from forehead_pattern_aggregate_solver import forehead_pattern_solve_aggregate, forehead_pattern_decompose_aggregate
from forehead_pattern_catalogue import forehead_pattern_catalogue, forehead_pattern_edges
from forehead_pattern_materialize import forehead_pattern_materialize

class ForeheadAggregateSyntheticTests(unittest.TestCase):

    def test_synthetic_invariants(self):
        ends = {str(i): (10 * i, 10 * i + 2) for i in range(5)}
        arcs = {(10 * i, 10 * i + 1) for i in range(5)} | {(10 * i + 1, 10 * i + 2) for i in range(5)}
        assert len(forehead_pattern_decompose_aggregate(ends, arcs, 3)) == 5
        bad = [arcs | {(100, 101), (101, 100)}, arcs - {(0, 1)}, arcs - {(1, 2), (11, 12)} | {(1, 12), (11, 2)}, arcs | {(0, 3)}, arcs | {(13, 12)}]
        for selected in bad:
            try:
                forehead_pattern_decompose_aggregate(ends, selected, 3)
            except ValueError:
                pass
            else:
                raise AssertionError('Invalid aggregate decomposition accepted')
        for seed in range(5):
            xyz = np.array([[float(x), 0.0, float(y)] for y in range(3) for x in range(11)])
            perm = np.random.default_rng(seed).permutation(len(xyz))
            inv = np.argsort(perm)
            v = xyz[inv].tolist()
            q0 = [[y * 11 + x, y * 11 + x + 1, (y + 1) * 11 + x + 1, (y + 1) * 11 + x] for y in range(2) for x in range(10)]
            faces = [[int(perm[i]) for i in q] for q in q0]
            paths = {str(x): [int(perm[y * 11 + x]) for y in range(3)] for x in (1, 3, 5, 7, 9)}
            edges = {e for q in faces for e in forehead_pattern_edges(q)}
            protected = [int(perm[16])]
            top = {'vertices': copy.deepcopy(v), 'chart': copy.deepcopy(v), 'faces': copy.deepcopy(faces), 'paths': copy.deepcopy(paths), 'lineage': {}, 'face_owners': list(range(len(faces))), 'edge_lineage': [{'edge': list(e), 'parent': list(e)} for e in edges], 'retiled_new_faces': [], 'retiled_old_faces': []}
            p = {'vertices': v, 'source_chart': v, 'faces': faces, 'paths': paths, 'topology': top, 'lineage': {}, 'protected': protected, 'moving': [], 'affected_new': [], 'affected_old': [], 'open_delta': [[0.0, 0.0, 0.0]] * len(v)}

            def sample(point):
                x, y = (point[0], point[2])
                ix = min(int(x), 9)
                iy = min(int(y), 1)
                u = x - ix
                w = y - iy
                ids = [int(perm[i]) for i in [iy * 11 + ix, iy * 11 + ix + 1, (iy + 1) * 11 + ix + 1, (iy + 1) * 11 + ix]]
                return (point.tolist(), {'donors': ids, 'weights': [(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w]})
            disc = {'role': [1] * len(v)}
            cat = forehead_pattern_catalogue(p, range(len(faces)), v, sample, disc, edges, {k: (path[0], path[-1]) for k, path in paths.items()}, allowed_protected_transits=protected)
            cat['patterns'] = [z for z in cat['patterns'] if 'transition_rotation' not in z]
            w = forehead_pattern_solve_aggregate(p, cat, time_limit=5, max_new_points=0, max_route_edges=3)
            assert w['feasible'], w
            out, a = forehead_pattern_materialize(p, w, v, p['open_delta'], disc, {e: (0.0, 0, False, True) for e in edges}, edges)
            assert out['paths'] == paths and out['vertices'] == v and (out['protected'] == protected) and a['guide_disjoint_no_elbows']
            assert w['aggregate_decomposition']['role_count'] == 5
