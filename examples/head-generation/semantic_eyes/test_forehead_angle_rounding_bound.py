"""Exact enclosure and local IEEE premises, without a fitter or native run."""
from fractions import Fraction
import itertools
import math
import unittest
import numpy as np
from forehead_angle_rounding_bound import (
    forehead_angle_binary32_fraction, forehead_angle_fraction_up,
    forehead_angle_rounding_radius, forehead_angle_sqrt_up,
    forehead_float32_angle_bound)


SYNTHETIC_CORNER = np.array([
    [1000.018, 1000., 1000.], [1000., 1000., 1000.],
    [999.9376524550624, 1000.0360108822479, 1000.]])


def independent_angle(points):
    u, v = points[0]-points[1], points[2]-points[1]
    if np.linalg.norm(u) == 0 or np.linalg.norm(v) == 0:
        raise ValueError('Collapsed independent angle')
    u, v = u/np.linalg.norm(u), v/np.linalg.norm(v)
    return math.degrees(math.atan2(np.linalg.norm(np.cross(u, v)), float(u@v)))


def checked_bound(points):
    smooth = independent_angle(points)
    saved = independent_angle(points.astype(np.float32).astype(float))
    return forehead_float32_angle_bound(points, smooth, saved)


class AngleRoundingBoundTests(unittest.TestCase):
    def test_exact_sqrt_upper_enclosure(self):
        values = [Fraction(0), Fraction(1), Fraction(1, 3), Fraction(10)**-300,
                  Fraction(10)**300, Fraction(2)**-2148, Fraction(2)**2048]
        for value in values:
            upper = forehead_angle_sqrt_up(value)
            self.assertGreaterEqual(upper*upper, value)
            lower = upper-Fraction(1, 1 << 128)
            self.assertTrue(lower < 0 or lower*lower < value)
        with self.assertRaises(ValueError):
            forehead_angle_sqrt_up(Fraction(-1))

    def test_binary64_upper_conversion_and_overflow(self):
        for value in (Fraction(0), Fraction(1, 3), Fraction(2)**-1100,
                      Fraction(1)+Fraction(2)**-53, Fraction(10)**300):
            upper = forehead_angle_fraction_up(value)
            self.assertGreaterEqual(Fraction.from_float(upper), value)
            if value:
                self.assertLess(Fraction.from_float(float(np.nextafter(upper, -np.inf))), value)
        for value in (Fraction(-1), Fraction(10)**400):
            with self.assertRaises(ValueError):
                forehead_angle_fraction_up(value)

    def test_all_exponent_boundaries_and_midpoint_ties(self):
        encodings = {0, 1, 2, 0x007ffffe, 0x007fffff, 0x00800000, 0x7f7ffffe}
        for exponent in range(1, 255):
            encodings.update(((exponent << 23)-1, exponent << 23, (exponent << 23)+1))
        encodings |= {bits | 0x80000000 for bits in tuple(encodings)}
        for bits in encodings:
            q = np.array(bits, np.uint32).view(np.float32)
            value = forehead_angle_binary32_fraction(bits)
            self.assertEqual(value, Fraction.from_float(float(q)))
            forehead_angle_rounding_radius(value, bits, float(q))
            for direction in (-np.inf, np.inf):
                with np.errstate(over='ignore'):
                    adjacent = float(np.nextafter(q, np.float32(direction)))
                if not math.isfinite(adjacent):
                    continue
                midpoint = float((value+Fraction.from_float(adjacent))/2)
                for x in (np.nextafter(midpoint, -np.inf), midpoint,
                          np.nextafter(midpoint, np.inf)):
                    narrowed = np.float32(x)
                    target_bits = int(narrowed.view(np.uint32))
                    if target_bits & 0x7fffffff == 0x7f7fffff:
                        continue
                    radius = forehead_angle_rounding_radius(
                        Fraction.from_float(float(x)), target_bits, float(narrowed))
                    self.assertGreaterEqual(radius, abs(Fraction.from_float(float(x))-
                        Fraction.from_float(float(narrowed))))
        self.assertEqual(forehead_angle_rounding_radius(Fraction(0), 0, 0.), Fraction(2)**-150)

    def test_non_nearest_ties_and_flush_are_rejected(self):
        cases = [(Fraction(1)+Fraction(3, 1 << 25), 0x3f800000, 1.),
                 (Fraction(1)+Fraction(1, 1 << 24), 0x3f800001, float(np.float32(1)+np.float32(2**-23))),
                 (Fraction(2)**-149, 0, 0.),
                 (Fraction(2)**-149, 1, 0.)]
        for args in cases:
            with self.subTest(args=args), self.assertRaises(ValueError):
                forehead_angle_rounding_radius(*args)
        for bits in (0x7f7fffff, 0xff7fffff, 0x7f800000, 0x7fc00000):
            with self.assertRaises(ValueError):
                forehead_angle_rounding_radius(Fraction(0), bits, 0.)

    def test_synthetic_corner_and_power_of_two_scaling(self):
        bound, detail = checked_bound(SYNTHETIC_CORNER)
        self.assertAlmostEqual(bound, .42063646039214314, places=12)
        self.assertGreater(bound, .01218)
        self.assertTrue(detail['current_candidate_only'])
        for power in (-100, -50, -10, 0, 10, 50, 100):
            other, _ = checked_bound(SYNTHETIC_CORNER*2.**power)
            self.assertEqual(other, bound)

    def test_measured_original_evaluator_difference_is_exactly_enclosed(self):
        bound, detail = forehead_float32_angle_bound(SYNTHETIC_CORNER, 100., 101.)
        measured = abs(Fraction.from_float(101.)-Fraction.from_float(100.))
        self.assertGreaterEqual(Fraction.from_float(bound), measured)
        self.assertEqual(bound, detail['original_evaluator_difference_degrees'])
        self.assertGreater(bound, detail['geometry_bound_degrees'])

    def test_error_box_corners_and_wide_random_cases(self):
        bound, _ = checked_bound(SYNTHETIC_CORNER)
        narrowed = SYNTHETIC_CORNER.astype(np.float32)
        radii = np.array([float(forehead_angle_rounding_radius(Fraction.from_float(float(x)),
            int(q.view(np.uint32)), float(q))) for x, q in zip(SYNTHETIC_CORNER.ravel(), narrowed.ravel())]).reshape(3, 3)
        baseline = independent_angle(SYNTHETIC_CORNER)
        for signs in itertools.product((-1., 1.), repeat=9):
            changed = SYNTHETIC_CORNER+radii*np.array(signs).reshape(3, 3)
            self.assertLessEqual(abs(independent_angle(changed)-baseline), bound)
        random = np.random.default_rng(49127)
        supported = 0
        for _ in range(1000):
            power = int(random.integers(-145, 120))
            center = random.uniform(-2, 2, 3)*2.**power
            offsets = random.normal(size=(2, 3))*2.**power*10**random.uniform(-8, 1)
            points = np.array([center+offsets[0], center, center+offsets[1]])
            try:
                result, info = checked_bound(points)
            except ValueError:
                continue
            supported += 1
            self.assertGreaterEqual(result, info['original_evaluator_difference_degrees'])
        self.assertGreater(supported, 500)

    def test_invalid_inputs_and_error_ball_reaching_zero_fail_closed(self):
        valid = np.array([[1., 0, 0], [0., 0, 0], [0., 1, 0]])
        bad = [valid[:2], np.zeros((3, 3)), np.full((3, 3), np.inf),
               valid+1e38, valid*1e39, valid*2.**-149]
        for points in bad:
            with self.subTest(points=points), self.assertRaises(ValueError):
                forehead_float32_angle_bound(points, 90., 90.)
        for angles in ((np.nan, 90.), (90., np.inf), (-1., 90.), (90., 181.)):
            with self.assertRaises(ValueError):
                forehead_float32_angle_bound(valid, *angles)


if __name__ == '__main__':
    unittest.main()
