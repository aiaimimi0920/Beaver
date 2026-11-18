"""Shared contact identities preserve genuine nonsmooth rail fallbacks."""
import unittest
import numpy as np
from forehead_complete_world_differential import (
    forehead_cd_pair_candidates, forehead_cd_rail_cv, forehead_cd_weighted_cv,
)
from forehead_finite_rail_gap import forehead_finite_rail_distance_matrix


def contact_fixture():
    a = np.array([.123, -.047, .011])
    b = np.array([.311, .052, .093])
    edge = b-a
    normal = np.cross(edge, [.3, .4, -.7])
    normal /= np.linalg.norm(normal)
    joint = a+.371*edge+.037*normal
    return np.array([a, b, joint+.09*normal+.12*edge,
                     joint, joint+.05*normal-.18*edge])


def scatter_contact(local, owners, size=5):
    result = np.zeros((size, 3))
    np.add.at(result, owners, local)
    return result


class ContactDifferentialTests(unittest.TestCase):
    def test_shared_endpoint_derivative_is_exactly_identical(self):
        points = contact_fixture()
        for scale in (1e-3, 1., 100.):
            a, b, left, joint, right = points*scale
            first = forehead_cd_pair_candidates(a, b, left, joint)[3]
            second = forehead_cd_pair_candidates(a, b, joint, right)[2]
            self.assertFalse(first[2] or second[2])
            np.testing.assert_array_equal(
                scatter_contact(first[1], [0, 1, 2, 3]),
                scatter_contact(second[1], [0, 1, 3, 4]))

    def test_shared_endpoint_on_first_rail_is_identical(self):
        a, b, left, joint, right = contact_fixture()
        first = forehead_cd_pair_candidates(left, joint, a, b)[1]
        second = forehead_cd_pair_candidates(joint, right, a, b)[0]
        np.testing.assert_array_equal(
            scatter_contact(first[1], [2, 3, 0, 1]),
            scatter_contact(second[1], [3, 4, 0, 1]))

    def test_large_translation_uses_only_local_differences(self):
        points = contact_fixture()[[0, 1, 2, 3]]
        for magnitude in (2.**28, 2.**40):
            translated = points+np.array([magnitude, -magnitude, magnitude])
            # Use the representable translated geometry as the reference;
            # adding a huge translation itself necessarily rounds coordinates.
            local = translated-translated[0]
            for order, candidate in (([0, 1, 2, 3], 3), ([2, 3, 0, 1], 1)):
                actual = forehead_cd_pair_candidates(*translated[order])[candidate]
                expected = forehead_cd_pair_candidates(*local[order])[candidate]
                self.assertEqual(actual[0], expected[0])
                np.testing.assert_array_equal(actual[1], expected[1])
                self.assertEqual(actual[2], expected[2])

    def test_shared_feature_matches_independent_distance_derivative(self):
        points = contact_fixture()
        pair = points[[0, 1, 2, 3]]
        _, gradient, invalid = forehead_cd_pair_candidates(*pair)[3]
        self.assertFalse(invalid)
        for col in range(pair.size):
            hi, lo = pair.copy(), pair.copy()
            hi.flat[col] += 1e-7
            lo.flat[col] -= 1e-7
            a = forehead_finite_rail_distance_matrix(hi[:2], hi[2:])[0, 0]
            b = forehead_finite_rail_distance_matrix(lo[:2], lo[2:])[0, 0]
            self.assertAlmostEqual((a-b)/2e-7, gradient.flat[col], delta=2e-8)

    def test_shared_feature_keeps_zero_variance_fallback(self):
        a, b, left, joint, right = contact_fixture()
        value, gradient, flags, detail = forehead_cd_rail_cv(
            np.array([a, b]), np.array([left, joint, right]))
        # These three minima coincide, so the independent zero-variance gate
        # must remain active even though duplicate-contact ambiguity is gone.
        self.assertEqual(detail['ambiguous_minima'], 0)
        self.assertTrue(detail['zero_variance'])
        self.assertTrue(flags)

    def test_shared_contact_roundoff_variance_stays_fallback(self):
        points = np.array([
            [-.022066088301393977, .01909193027760145, .02027210081528158],
            [.08431798137650337, .11745421067489602, .21306414551218938],
            [.1159412670920766, .03007958129322577, .0906513272779596],
            [.024567108991052647, .05426111982269585, .09253328154284103],
            [.04908912630440683, .01656423591943537, .04393238037044743]])
        for scale in (1e-3, 1., 1e3):
            for offset in (-1e-7, 0., 1e-7):
                changed = points.copy()
                changed[3, 2] += offset
                value, gradient, flags, detail = forehead_cd_rail_cv(
                    changed[:2]*scale, changed[2:]*scale)
                self.assertLess(value, 1e-13)
                self.assertTrue(detail['zero_variance'])
                self.assertEqual(flags, set(range(5)))
                np.testing.assert_array_equal(gradient, np.zeros((5, 3)))

    def test_tiny_common_gap_keeps_structural_zero_variance_fallback(self):
        a, b = contact_fixture()[:2]
        edge = b-a
        normal = np.cross(edge, [.3, .4, -.7])
        normal /= np.linalg.norm(normal)
        for gap in (1e-6, 1e-8, 1e-10, 1e-12, 1e-14):
            joint = a+.371*edge+gap*normal
            rail = np.array([joint+.09*normal+.12*edge,
                             joint, joint+.05*normal-.18*edge])
            _, gradient, flags, detail = forehead_cd_rail_cv(np.array([a, b]), rail)
            self.assertTrue(detail['identical_minimum_gradients'])
            self.assertTrue(detail['zero_variance'])
            self.assertEqual(flags, set(range(5)))
            np.testing.assert_array_equal(gradient, np.zeros((5, 3)))

    def test_cv_roundoff_band_is_conservative_and_scale_aware(self):
        for scale in (1e-12, 1., 1e12):
            for spread in (0., 1e-15, 1e-14):
                value, dx, dw = forehead_cd_weighted_cv(
                    scale*np.array([1-spread, 1., 1+spread]), np.array([1., 2., 3.]))
                self.assertIsNone(dx)
                self.assertIsNone(dw)
                self.assertGreaterEqual(value, 0)
            _, dx, dw = forehead_cd_weighted_cv(
                scale*np.array([.99, 1., 1.01]), np.array([1., 2., 3.]))
            self.assertTrue(np.isfinite(dx).all())
            self.assertTrue(np.isfinite(dw).all())

    def test_distinct_equal_contacts_stay_fallback_and_branch_changes(self):
        a = np.array([[0., 0, 0], [4., 0, 0]])
        b = np.array([[1., 1, 0], [2., 2, 0], [3., 1, 0],
                      [3.5, 2.3, 0], [3.7, 3., .1]])
        _, _, flags, detail = forehead_cd_rail_cv(a, b)
        self.assertGreater(detail['ambiguous_minima'], 0)
        self.assertFalse(detail['zero_variance'])
        self.assertTrue(flags)
        gradients = []
        for sign in (-1, 1):
            changed = b.copy()
            changed[0, 1] += sign*1e-4
            changed[2, 1] -= sign*1e-4
            _, gradient, flags, detail = forehead_cd_rail_cv(a, changed)
            self.assertFalse(flags)
            self.assertEqual(detail['ambiguous_minima'], 0)
            gradients.append(gradient)
        self.assertGreater(np.linalg.norm(gradients[0]-gradients[1]), .01)

    def test_parallel_and_near_parallel_remain_unsupported(self):
        for slope in (0., 1e-10):
            a, b = np.array([0., 0, 0]), np.array([1., 0, 0])
            c, d = np.array([0., 1, 0]), np.array([1., 1+slope, 0])
            self.assertTrue(all(row[2] for row in forehead_cd_pair_candidates(a, b, c, d)))

    def test_collapsed_and_zero_distance_are_not_made_analytic(self):
        with self.assertRaisesRegex(ValueError, 'Collapsed'):
            forehead_cd_pair_candidates(np.zeros(3), np.zeros(3),
                                        np.ones(3), np.array([2., 1, 1]))
        pairs = forehead_cd_pair_candidates(np.array([0., 0, 0]), np.array([1., 0, 0]),
                                           np.array([0., 0, 0]), np.array([0., 1, 0]))
        self.assertTrue(any(value == 0 and gradient is None
                            for value, gradient, _ in pairs))

    def test_random_primal_and_selected_smooth_derivatives(self):
        rng = np.random.default_rng(894)
        checked = 0
        for _ in range(60):
            points = rng.normal(size=(4, 3))
            candidates = forehead_cd_pair_candidates(*points)
            selected = min(candidates, key=lambda item: item[0])
            expected = forehead_finite_rail_distance_matrix(points[:2], points[2:])[0, 0]
            self.assertAlmostEqual(selected[0], expected, delta=2e-14)
            value, gradient, invalid = selected
            if invalid or gradient is None:
                continue
            direction = rng.normal(size=points.shape)
            direction /= np.linalg.norm(direction)
            hi, lo = points+1e-7*direction, points-1e-7*direction
            numeric = (forehead_finite_rail_distance_matrix(hi[:2], hi[2:])[0, 0]
                       - forehead_finite_rail_distance_matrix(lo[:2], lo[2:])[0, 0])/2e-7
            self.assertAlmostEqual(numeric, np.sum(gradient*direction), delta=2e-7)
            checked += 1
        self.assertEqual(checked, 60)


if __name__ == '__main__':
    unittest.main()
