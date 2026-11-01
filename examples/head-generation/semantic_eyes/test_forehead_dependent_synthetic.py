"""Corpus-free synthetic regression; no model IDs, files, native calls or baseline resets."""
import unittest
import copy
import numpy as np
from forehead_birth_geometry import forehead_birth_solve, forehead_birth_dependency_order

class ForeheadDependentSyntheticTests(unittest.TestCase):

    def test_synthetic_invariants(self):
        deps, order = forehead_birth_dependency_order({7: (6, 1, 0.25), 6: (0, 1, 0.5)}, 8, 6)
        assert order == [6, 7]
        for bad in ({6: (7, 0, 0.5), 7: (6, 1, 0.5)}, {3: (0, 1, 0.5)}, {6: (6, 0, 0.5)}, {6: (0, 1, 1.0)}):
            try:
                forehead_birth_dependency_order(bad, 8, 6)
            except ValueError:
                pass
            else:
                raise AssertionError('Invalid dependent graph accepted')

        def run(seed, gradient=0.0, chain=False, bad_source_parameters=False, front_cap=None):
            original = np.array([[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 0.5], [0, 0, 1.5], [1, 0, 1.5]], float)
            perm = np.random.default_rng(seed).permutation(6)
            inverse = np.argsort(perm)
            source = original[inverse]
            v = np.vstack([source, [[0, 0, 1.0], [0, 0, 1.25]] if chain else [[0, 0, 1.0]]])
            params = v[:, [0, 2]]
            oldfield = np.zeros((6, 3))
            oldfield[:, 2] = source[:, 2] * gradient
            current = np.zeros((len(v), 3))
            current[:6] = oldfield
            current[6, 2] = gradient
            oldcopy = oldfield.copy()
            vcopy = v.copy()
            discrete = {'role': [[1, 0] for i in range(6)]}

            def sample(q):
                return np.column_stack([q[:, 0], np.zeros(len(q)), q[:, 1]])

            def field(q):
                return np.column_stack([np.zeros((len(q), 2)), q[:, 1] * gradient])

            def row(q):
                u = q[0]
                z = q[1] / 1.5
                weights = np.maximum([(1 - u) * (1 - z), u * (1 - z), u * z, (1 - u) * z], 0.0)
                weights /= sum(weights)
                return {'donors': [int(perm[i]) for i in [0, 1, 5, 4]], 'weights': weights.tolist()}
            result = forehead_birth_solve(vertices=v, faces=[[int(perm[i]) for i in [0, 1, 2]] + [6], [6, int(perm[2]), int(perm[5]), 7]] if chain else [[int(perm[i]) for i in [0, 1, 2]] + [6]], paths={'rail': [int(perm[0]), 7 if chain else 6]}, parameters=params, existing_count=6, frozen_old_fields=oldfield, current_fields=current, source_sampler=sample, field_sampler=field, row_sampler=row, source_vertices=source, source_fields=oldfield, source_parameters=source[:, [0, 2]] + (0.01 if bad_source_parameters else 0.0), actual_chart=lambda points: points[:, [0, 2]], movable_old=[int(perm[3])], held_nodes=[int(perm[i]) for i in [0, 1, 2, 4, 5]], free_births=[], fixed_births=[], dependent_births={6: (int(perm[3]), int(perm[4]), 0.5), 7: (6, int(perm[4]), 0.5)} if chain else {6: (int(perm[3]), int(perm[4]), 0.5)}, active_roles=['rail'], guard_face_ids=[0, 1] if chain else [0], rail_targets={int(perm[3]): [0, 0.7]}, front_caps=None if front_cap is None else {'rail': front_cap}, quality_margin=0.0 if front_cap == 180.0 else 0.02, target_tolerance_mm=0.01, old_radius_mm=300, phase_evaluations=(60, 100), wall_seconds=10, source_discrete=discrete, discrete_reference=discrete)
            assert np.array_equal(v, vcopy) and np.array_equal(oldfield, oldcopy)
            if gradient or bad_source_parameters or (front_cap is not None and front_cap < 180):
                assert not result['admitted']
                assert 'new_birth_rows' not in result
                return
            assert result['admitted'], result
            p = result['chosen_parameters']
            assert np.linalg.norm(np.array(p[6]) - 0.5 * (np.array(p[int(perm[3])]) + params[int(perm[4])])) < 1e-12
            assert abs(result['positions'][6][2] - 1.1) < 0.0001
            assert result['old_baselines_untouched']
            if chain:
                assert abs(result['positions'][7][2] - 1.3) < 0.0001 and np.linalg.norm(np.array(p[7]) - 0.5 * (np.array(p[6]) + params[int(perm[4])])) < 1e-12
            return result
        for seed in range(5):
            run(seed)
        run(10, 0.01)
        run(20, chain=True)
        run(30, bad_source_parameters=True)
        run(40, front_cap=180.0)
        run(41, front_cap=100.0)
