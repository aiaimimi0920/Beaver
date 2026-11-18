"""Whole-path evidence must not conceal gradual density or folded-back tails."""
import unittest
import numpy as np
from forehead_spacing import forehead_station_residuals, measure_forehead_station_spacing, forehead_frozen_field, forehead_initial_parameters, forehead_physical_turn_residuals, forehead_uniform_stage_target
from forehead_surface_batch import make_forehead_surface_batch
from forehead_gap_measure import measure_forehead_transverse_spacing


class ForeheadSpacingTests(unittest.TestCase):
    def line(self, lengths):
        x = np.r_[0., np.cumsum(lengths)]
        return np.column_stack((x, x*0, x*0))

    def test_uniform(self):
        points = self.line(np.full(20, .01))
        np.testing.assert_allclose(forehead_station_residuals(points, {'red': range(21)}), 0, atol=1e-14)

    def test_physical_turn_objective_does_not_hide_bend_by_subdivision(self):
        points = np.array([[0,0,0],[.04,0,0],[.04,.04,0]])
        refined = np.array([[0,0,0],[.02,0,0],[.04,0,0],[.04,.02,0],[.04,.04,0]])
        raw = forehead_physical_turn_residuals(points,{'orange':range(3)},{'orange':20.})
        split = forehead_physical_turn_residuals(refined,{'orange':range(5)},{'orange':20.})
        np.testing.assert_allclose(raw,split,atol=1e-10)
        self.assertGreater(np.linalg.norm(raw),0)
        straight = self.line([.02,.02,.02,.02])
        np.testing.assert_allclose(forehead_physical_turn_residuals(
            straight,{'red':range(5)},{'red':18.}),0)

    def test_large_uniform_redistribution_is_bounded_but_not_redefined_as_uniform(self):
        points = self.line([.001,.099])
        target, report = forehead_uniform_stage_target(points,points,points,{},
            {'red':[0,1,2]},[1],[[0,1,2]],{0,2},max_step_mm=6.)
        np.testing.assert_allclose(target,[7.,0.],atol=1e-12)
        self.assertAlmostEqual(report['full_uniform_target_maximum_mm'],49.)
        self.assertLess(report['stage_fraction'],1.)
        self.assertTrue(report['partial_step_is_not_equal_spacing_acceptance'])
        final, report = forehead_uniform_stage_target(points,points,points,{},
            {'red':[0,1,2]},[1],[[0,1,2]],{0,2},max_step_mm=100.)
        np.testing.assert_allclose(final,[50.,0.],atol=1e-12)
        self.assertEqual(report['stage_fraction'],1.)

    def test_gradient_is_not_equality(self):
        points = self.line(.002*1.07**np.arange(30))
        result = measure_forehead_station_spacing(points, {'green': range(31)})['green']
        self.assertGreater(result['max_min_ratio'], 7)
        self.assertGreater(result['coefficient_of_variation'], .5)

    def test_scale_and_direction(self):
        points = self.line([.01, .012, .009, .01])
        residual = forehead_station_residuals(points, {'blue': range(5)})
        np.testing.assert_allclose(residual, forehead_station_residuals(points*3+2, {'blue': range(5)}))
        np.testing.assert_allclose(residual[::-1], forehead_station_residuals(points, {'blue': range(4, -1, -1)}))

    def test_frozen_original_field_cannot_reset(self):
        old = np.array([[0., 0., 0.], [.001, 0., 0.]])
        lineage = {0: {'donors': [0, 1], 'weights': [.3, .7]},
                   2: {'donors': [0, 1], 'weights': [.5, .5]}}
        frozen = forehead_frozen_field(old, 3, lineage)
        np.testing.assert_array_equal(frozen[:2], old)
        np.testing.assert_allclose(frozen[2], [.0005, 0, 0])

    def test_lineage_parameters_reproduce_bilinear_world_sample(self):
        angle, radius = .8, .1
        old = np.array([[0.,-radius,1.6], [radius*np.sin(angle),-radius*np.cos(angle),1.6],
                        [radius*np.sin(angle),-radius*np.cos(angle),1.7], [0.,-radius,1.7]])
        chart = np.column_stack((radius*np.arctan2(old[:,0],-old[:,1]), old[:,0]*0, old[:,2]))
        weights = np.array([.18,.42,.28,.12])
        points = np.vstack((old, weights@old))
        lineage = {4: {'weights': weights, 'donors': [0,1,2,3]}}
        parameters = forehead_initial_parameters(old,points,chart,lineage,[4])
        sampled = make_forehead_surface_batch(chart,old,np.array([[0,1,2,3]])).sample(parameters)
        np.testing.assert_allclose(sampled[0],points[4],atol=1e-12)
        inferred = radius*np.arctan2(points[4,0],-points[4,1])
        self.assertGreater(abs(inferred-parameters[0,0]), .0001)
        with self.assertRaises(ValueError):
            forehead_initial_parameters(old,points,chart,{},[4])

    def five_curves(self, curves):
        names = ('red', 'orange', 'green', 'blue', 'purple')
        paths, offset = {}, 0
        for name, curve in zip(names, curves):
            paths[name] = list(range(offset, offset+len(curve)))
            offset += len(curve)
        return np.vstack(curves), paths

    def test_equal_parallel_curves(self):
        curve = self.line(np.full(20, .01))
        points, paths = self.five_curves([curve+[0, 0, i*.02] for i in range(5)])
        result = measure_forehead_transverse_spacing(points, paths)
        self.assertAlmostEqual(result['max_min_ratio'], 1., places=10)
        self.assertTrue(all(row['all_samples_retained'] for row in result['directions']))

    def test_short_unrelated_guide_cannot_hide_long_pair_tail(self):
        curves = []
        for i in range(5):
            curve = self.line(np.full(30 if i < 2 else 20, .01))
            curve[:, 2] = i*.02
            if i == 1:
                curve[:, 2] -= .15*np.maximum(curve[:, 0]-.2, 0)
            curves.append(curve)
        points, paths = self.five_curves(curves)
        result = measure_forehead_transverse_spacing(points, paths)
        pair = next(row for row in result['directions'] if row['source']=='orange' and row['target']=='red')
        self.assertLess(pair['minimum_mm'], 5.1)
        self.assertGreater(pair['max_min_ratio'], 3.9)

    def test_angular_return_and_unmatched_tails_are_not_discarded(self):
        base = np.array([[0., -.1, 1.8], [.1, -.05, 1.8], [.08, -.07, 1.6]])
        points, paths = self.five_curves([base+[0, 0, i*.02] for i in range(5)])
        result = measure_forehead_transverse_spacing(points, paths)
        self.assertTrue(any(row['beyond_target_endpoint_samples'] for row in result['directions']))
        for row in result['directions']:
            self.assertAlmostEqual(row['source_arc_length_mm'], np.linalg.norm(np.diff(base, axis=0), axis=1).sum()*1000)
            self.assertTrue(row['all_samples_retained'])

    def test_invalid_guides_fail_closed(self):
        for lengths in ([.01, 0, .01], [.01, float('nan'), .01]):
            with self.assertRaises(ValueError):
                forehead_station_residuals(self.line(lengths), {'purple': range(4)})


if __name__ == '__main__':
    unittest.main()
