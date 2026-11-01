"""State-machine tests; solver success is mocked, never a geometry claim."""
import copy
import json
import unittest
import numpy as np
from collections import UserDict
from unittest.mock import patch
from forehead_flow_continuation import continue_forehead_flows, forehead_flow_progress, forehead_continuation_equal, forehead_continuation_retains, forehead_continuation_failure_diagnostic


class ForeheadFlowContinuationTests(unittest.TestCase):
    def setUp(self):
        names = ('red', 'orange', 'green', 'blue', 'purple')
        vertices = [[0., 0., 0.], [.001, 0., 0.], [.002, 0., 0.]]
        self.author = {'vertices': vertices, 'open_delta': [[0., 0., 0.]]*3}
        self.proposal = {'vertices': vertices, 'paths': {n: [0, 1, 2] for n in names},
                         'uniform_station_reference_length_mm': {n: 2. for n in names}}

    def test_optional_seam_growth_forwarding_and_default(self):
        for options, expected in (({}, False), ({'seam_active_set_growth': True}, True)):
            with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=ValueError('diagnostic')) as fit:
                result = continue_forehead_flows(self.author, self.proposal, **options)
            self.assertTrue(fit.call_args_list)
            self.assertTrue(all(c.kwargs['seam_active_set_growth'] is expected for c in fit.call_args_list))
            self.assertFalse(result['full_model_success'])
        with self.assertRaises(ValueError):
            continue_forehead_flows(self.author, self.proposal, seam_active_set_growth=1)

    def test_missing_reference_is_not_reset(self):
        del self.proposal['uniform_station_reference_length_mm']
        with self.assertRaisesRegex(ValueError, 'frozen initial'):
            forehead_flow_progress(self.author, self.proposal)

    def test_failed_trial_cannot_mutate_accepted_state(self):
        before = copy.deepcopy(self.proposal)
        def reject(*args, **kwargs):
            args[2]['vertices'][0][0] = 999.
            raise ValueError('Not feasible')
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=reject):
            result = continue_forehead_flows(self.author, self.proposal)
        self.assertEqual(self.proposal, before)
        self.assertEqual(result['proposal'], before)
        self.assertEqual(len(result['attempts']), 5)
        self.assertFalse(result['flow_goals_passed'])
        self.assertFalse(result['full_model_success'])

    def test_reference_reset_rejected(self):
        def reset(*args, **kwargs):
            candidate = args[2]
            candidate['uniform_station_reference_length_mm']['blue'] = 99.
            return candidate
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['blue'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=reset):
            result = continue_forehead_flows(self.author, self.proposal)
        self.assertEqual(result['proposal'], self.proposal)
        self.assertIn('changed frozen', result['attempts'][0]['error'])

    def test_checkpoint_is_detached_and_success_is_only_flow_success(self):
        def malicious_callback(state):
            state['proposal']['vertices'][0][0] = 999.
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, [])):
            result = continue_forehead_flows(self.author, self.proposal, checkpoint=malicious_callback)
        self.assertEqual(result['proposal'], self.proposal)
        self.assertTrue(result['flow_goals_passed'])
        self.assertFalse(result['full_model_success'])

    def test_nonfinite_author_and_appended_front_fail_closed(self):
        bad = copy.deepcopy(self.author);bad['vertices'][1][2] = float('nan')
        with self.assertRaisesRegex(ValueError, 'Nonfinite authored'):
            forehead_flow_progress(bad, self.proposal)
        p = copy.deepcopy(self.proposal);p['vertices'].append([.0015, 0., 0.])
        p['paths']['red'] = [0, 3, 2]
        with self.assertRaisesRegex(ValueError, 'original first-two'):
            forehead_flow_progress(self.author, p)

    def test_front_and_arc_alone_keep_role_active(self):
        p = copy.deepcopy(self.proposal)
        with patch('forehead_flow_continuation.measure_forehead_flow', return_value={'sampled_peak_degrees': 0.}):
            p['uniform_station_reference_length_mm']['red'] = 1.
            metrics, active = forehead_flow_progress(self.author, p)
            self.assertIn('red', active);self.assertEqual(metrics['red']['original_length_inflation'], 2.)
            p = copy.deepcopy(self.proposal);p['vertices'] = [[0., 0., 0.], [.001, 0., .001], [.002, 0., .002]]
            p['uniform_station_reference_length_mm'] = {n: 3. for n in p['paths']}
            metrics, active = forehead_flow_progress(self.author, p)
            self.assertEqual(set(active), set(p['paths']));self.assertEqual(metrics['red']['front_degrees'], 90.)

    def test_rejection_isolates_author_arrays_and_selected_list(self):
        before = copy.deepcopy(self.author)
        def reject(*args, **kwargs):
            args[0][0][0] = 999.
            kwargs['author_context']['vertices'][0][0] = 999.
            kwargs['progress_names'].clear()
            raise ValueError('Rejected')
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=reject):
            result = continue_forehead_flows(self.author, self.proposal)
        self.assertEqual(self.author, before)
        self.assertTrue(all(a['roles'] == ['orange'] for a in result['attempts']))

    def test_baseline_or_other_caps_cannot_be_changed(self):
        p = copy.deepcopy(self.proposal)
        p['topology'] = {'lineage': {'2': {'donors': [2], 'weights': [1.]}}}
        p['matched_gap_caps_v2'] = {'pair:chart': .2}
        p['joint_stage_caps'] = {'front': {n: 8. for n in p['paths']}}
        for what in ('baseline', 'gaps', 'unselected', 'loosen'):
            def bad(*args, **kwargs):
                candidate = args[2]
                if what == 'baseline':candidate['topology']['lineage']['2']['weights'] = [.9]
                if what == 'gaps':candidate['matched_gap_caps_v2']['pair:chart'] = .3
                if what == 'unselected':candidate['joint_stage_caps']['front']['blue'] = 7.
                if what == 'loosen':candidate['joint_stage_caps']['front']['orange'] = 9.
                return candidate
            with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=bad):
                result = continue_forehead_flows(self.author, p)
            self.assertEqual(result['proposal'], p);self.assertEqual(len(result['stages']), 0)

    def test_recursive_numpy_and_lineage_representation_retained(self):
        p = copy.deepcopy(self.proposal)
        p['source_chart'] = [[0., 0., 0.], [1., 0., 0.]]
        p['protected'] = [0]
        p['topology'] = {'chart': p['source_chart'], 'lineage': {'2': {'donors': [0, 1], 'weights': [.25, .75]}}}
        q = copy.deepcopy(p)
        q['source_chart'] = np.asarray(q['source_chart'])
        q['vertices'] = np.asarray(q['vertices'])
        q['protected'] = np.array([0], dtype=np.int64)
        q['paths'] = {name: np.asarray(path) for name, path in q['paths'].items()}
        q['uniform_station_reference_length_mm'] = UserDict({name: np.float64(v) for name, v in q['uniform_station_reference_length_mm'].items()})
        q['topology']['chart'] = np.asarray(q['topology']['chart'])
        q['topology']['lineage'] = {np.int64(2): {'donors': np.array([0, 1]), 'weights': np.array([.25, .75])}}
        forehead_continuation_retains(p, q, ['orange'])
        forehead_continuation_retains(q, copy.deepcopy(q), ['orange'])
        q['topology']['lineage'][2]['weights'][0] += .001
        with self.assertRaisesRegex(ValueError, 'field baseline'):
            forehead_continuation_retains(p, q, ['orange'])

    def test_comparison_exact_and_key_normalization_scoped(self):
        self.assertTrue(forehead_continuation_equal({'v': (np.int64(1), np.array([2., 3.]))}, {'v': [1, [2., 3.]]}))
        self.assertTrue(forehead_continuation_equal(np.array(2.), np.float64(2.)))
        self.assertFalse(forehead_continuation_equal({'2': 3}, {2: 3}))
        self.assertTrue(forehead_continuation_equal({'2': [1.]}, {2: np.array([1.])}, lineage=True))
        self.assertFalse(forehead_continuation_equal({'2': [1.], 2: [1.]}, {2: [1.]}, lineage=True))
        self.assertFalse(forehead_continuation_equal(np.array([1.]), np.array([1.0000000001])))
        self.assertFalse(forehead_continuation_equal(np.array([float('nan')]), [float('nan')]))
        self.assertFalse(forehead_continuation_equal(np.ones((1, 2)), np.ones(2)))

    def test_bounded_step_exhaustion_is_not_completion(self):
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['purple'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=lambda *a, **k: a[2]):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=1)
        self.assertEqual(len(result['stages']), 1)
        self.assertIn('budget exhausted', result['stop_reason'])
        self.assertFalse(result['flow_goals_passed'])

    def test_compact_residual_groups_and_worst_rows(self):
        raw = np.r_[np.linspace(-2., 2., 40), np.nan, np.inf]
        payload = {'stage': 'solver_end', 'raw_constraints': raw,
                   'constraint_scales': np.ones(42), 'vertices': np.zeros((100, 3)),
                   'constraint_layout': {'angle_lower': (0, 10), 'aspect': (10, 30),
                                         'field': (30, 42), 'count': 42}}
        diagnostic = forehead_continuation_failure_diagnostic(payload)
        self.assertNotIn('raw_constraints', diagnostic)
        self.assertNotIn('constraint_scales', diagnostic)
        self.assertNotIn('vertices', diagnostic)
        self.assertEqual(diagnostic['raw_summary']['failure_count'], 22)
        self.assertEqual(diagnostic['constraint_groups']['aspect']['negative_count'], 10)
        self.assertEqual(diagnostic['constraint_groups']['field']['nonfinite_count'], 2)
        self.assertEqual(diagnostic['constraint_groups']['angle_lower']['minimum_finite'], -2.)
        self.assertEqual(len(diagnostic['worst_raw_rows']), 24)
        self.assertEqual([r['index'] for r in diagnostic['worst_raw_rows'][:2]], [40, 41])
        json.dumps(diagnostic, allow_nan=False)

    def test_failure_capture_detached_and_never_success_upgrade(self):
        before = copy.deepcopy(self.proposal)
        def reject(*args, **kwargs):
            payload = {'stage': 'solver_end', 'variable_ids': np.array([1]),
                       'start_parameters_mm': np.array([1., 2.]),
                       'parameters_mm': np.array([3., 4.]),
                       'constraint_layout': {'aspect': (0, 2), 'count': 2},
                       'raw_constraints': np.array([-.2, .1]), 'solver_success': True,
                       'solver_message': 'Optimizer terminated, final gate failed',
                       'lower_turn_guard': {'triples': np.array([[0, 1, 2]])},
                       'restoration_continuation': {'attempted': True,
                                                   'raw_constraints': [-999.]}}
            kwargs['diagnostic'](payload)
            kwargs['diagnostic']({'stage': 'minimum_improvement_unmet',
                                  'parameters_mm': np.array([5., 6.]),
                                  'geometry_admission_passed': True})
            payload['start_parameters_mm'][0] = 999.
            payload['lower_turn_guard']['triples'][0, 0] = 999
            args[2]['vertices'][0][0] = 999.
            raise ValueError('Original final admission error')
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=reject):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=1)
        self.assertEqual(result['proposal'], before)
        self.assertEqual(self.proposal, before)
        self.assertEqual(len(result['attempts']), 5)
        self.assertEqual(result['stages'], [])
        self.assertFalse(result['full_model_success'])
        for attempt in result['attempts']:
            self.assertEqual(attempt['error'], 'Original final admission error')
            d = attempt['solver_diagnostic']
            self.assertEqual(d['variable_ids'], [1])
            self.assertEqual(d['start_parameters_mm'], [1., 2.])
            self.assertEqual(d['parameters_mm'], [5., 6.])
            self.assertEqual(d['lower_turn_guard']['triples'], [[0, 1, 2]])
            self.assertEqual(d['constraint_groups']['aspect']['failure_count'], 1)
            self.assertTrue(d['solver_success'])
            self.assertFalse(d['admitted'])
            self.assertFalse(d['full_model_success'])
            self.assertNotIn('raw_constraints', d['restoration_continuation'])
        result['attempts'][0]['solver_diagnostic']['parameters_mm'][0] = 999.
        self.assertEqual(result['attempts'][1]['solver_diagnostic']['parameters_mm'], [5., 6.])

    def test_success_diagnostics_discarded_and_observer_error_isolated(self):
        def success(*args, **kwargs):
            kwargs['diagnostic']({'raw_constraints': [[1.]], 'stage': 'malformed'})
            kwargs['diagnostic']({'stage': 'solver_end', 'raw_constraints': [1.]})
            return args[2]
        with patch('forehead_flow_continuation.forehead_flow_progress', side_effect=[({}, ['orange']), ({}, [])]), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=success):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=1)
        self.assertEqual(result['attempts'], [])
        self.assertEqual(len(result['stages']), 1)
        self.assertNotIn('solver_diagnostic', result['stages'][0])
        self.assertTrue(result['flow_goals_passed'])
        self.assertFalse(result['full_model_success'])

    def test_preflight_error_without_solver_payload_unchanged(self):
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=ValueError('Preflight failed')):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=1)
        self.assertTrue(all(a['error'] == 'Preflight failed' for a in result['attempts']))
        self.assertTrue(all('solver_diagnostic' not in a for a in result['attempts']))

    def test_slow_small_success_is_reused_and_persisted(self):
        scales = []
        def fit(*args, **kwargs):
            scale = kwargs['progress_scale'];scales.append(scale)
            if scale > .25: raise ValueError('Full and half blocked')
            candidate = args[2];candidate['metrics'] = {'solver_iteration_budget': 120, 'selected_feasible_iteration': 4}
            return candidate
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=fit):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=2)
            resumed = continue_forehead_flows(self.author, result['proposal'], max_steps=1)
        self.assertEqual(scales, [1., .5, .25, .25, .25])
        self.assertEqual(resumed['step_schedule']['orange']['next_scale'], .25)
        self.assertNotIn('flow_continuation_schedule', self.proposal)
        self.assertFalse(result['full_model_success'])

    def test_fast_success_increases_only_one_level(self):
        p = copy.deepcopy(self.proposal)
        p['metrics'] = {'physical_flow_stage': {'step_scale': .25, 'tightened_curves': ['orange']}, 'solver_iterations': 15}
        scales = []
        def fit(*args, **kwargs):
            scales.append(kwargs['progress_scale']);q = args[2]
            q['metrics'] = {'solver_iterations': 15};return q
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=fit):
            result = continue_forehead_flows(self.author, p, max_steps=2)
        self.assertEqual(scales, [.5, 1.])
        self.assertEqual(result['stages'][0]['scheduling_before']['orange']['initialized_from'], 'physical_flow_stage')

    def test_role_schedule_independent_and_group_uses_smallest(self):
        from forehead_flow_continuation import forehead_continuation_step_schedule
        p = copy.deepcopy(self.proposal)
        p['flow_continuation_schedule'] = {'blue': {'next_scale': .25, 'reason': 'Prior slow admission'}}
        p['metrics'] = {'physical_flow_stage': {'step_scale': .5, 'tightened_curves': ['orange']}, 'solver_iteration_budget': 120}
        schedule = forehead_continuation_step_schedule(p)
        self.assertEqual([schedule[n]['next_scale'] for n in ('red', 'orange', 'blue', 'purple')], [1., .5, .25, 1.])
        scales = []
        def fit(*args, **kwargs):
            scales.append(kwargs['progress_scale']);return args[2]
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['blue', 'purple'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=fit):
            result = continue_forehead_flows(self.author, p, max_steps=1)
        self.assertEqual(scales, [.25])
        self.assertEqual(result['step_schedule']['orange']['next_scale'], .5)

    def test_unknown_or_restored_effort_never_promotes(self):
        from forehead_flow_continuation import forehead_continuation_step_record
        for metrics in ({}, {'selected_feasible_iteration': 1}, {'solver_iterations': True},
                        {'solver_iterations': 3, 'restoration_continuation': {'attempted': True}},
                        {'solver_iterations': 16}, {'solver_iterations': 5, 'solver_work_iterations': 20}):
            self.assertEqual(forehead_continuation_step_record(.25, metrics)['next_scale'], .25)
        self.assertEqual(forehead_continuation_step_record(.25, {'solver_iteration_budget': 15})['next_scale'], .5)

    def test_rejected_small_trial_does_not_update_history(self):
        p = copy.deepcopy(self.proposal)
        p['metrics'] = {'physical_flow_stage': {'step_scale': .5, 'tightened_curves': ['orange']}}
        before = copy.deepcopy(p)
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=ValueError('Rejected')):
            result = continue_forehead_flows(self.author, p, max_steps=1)
        self.assertEqual([a['scale'] for a in result['attempts']], [.5, .25, .1, .05])
        self.assertEqual(p, before);self.assertEqual(result['proposal'], before)
        self.assertEqual(result['step_schedule']['orange']['next_scale'], .5)


    def test_fine_scale_recovery_and_one_level_promotion(self):
        from forehead_flow_continuation import forehead_continuation_step_record, forehead_continuation_step_schedule
        p = copy.deepcopy(self.proposal)
        p['metrics'] = {'physical_flow_stage': {'step_scale': .05, 'tightened_curves': ['orange']}}
        self.assertEqual(forehead_continuation_step_schedule(p)['orange']['next_scale'], .05)
        self.assertEqual(forehead_continuation_step_record(.05, {'solver_iterations': 10})['next_scale'], .1)
        self.assertEqual(forehead_continuation_step_record(.1, {'solver_iterations': 10})['next_scale'], .25)
        with self.assertRaises(ValueError):
            forehead_continuation_step_record(.2, {})


    def test_blocked_group_does_not_starve_other_active_roles(self):
        calls = []
        def fit(*args, **kwargs):
            calls.append(tuple(kwargs['progress_names']))
            if kwargs['progress_names'] == ['orange']:
                raise ValueError('Orange local capacity blocked')
            return args[2]
        with patch('forehead_flow_continuation.forehead_flow_progress', return_value=({}, ['orange', 'blue', 'purple'])), patch('forehead_flow_continuation.fit_forehead_shared_constraints', side_effect=fit):
            result = continue_forehead_flows(self.author, self.proposal, max_steps=2)
        self.assertEqual(calls[:5], [('orange',)] * 5)
        self.assertEqual(calls[-1], ('blue', 'purple'))
        self.assertEqual(len(result['stages']), 1)
        self.assertFalse(result['flow_goals_passed'])


if __name__ == '__main__':
    unittest.main()
