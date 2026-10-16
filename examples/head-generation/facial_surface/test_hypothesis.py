import math
import unittest
from hypothesis import validate, base_correction, orbital_support, outer_collar


class HypothesisTests(unittest.TestCase):
    def test_contract_and_neutral(self):
        p = validate({})
        self.assertEqual(base_correction(0.5, 0.4, 0.236, 0.08, -0.12, 0.079, p), 0)
        self.assertEqual(orbital_support(0.02, 0.015, 0, p), (0.02, 0.015))
        for bad in (
            {"unknown": 1},
            {"orbit_shallowing": True},
            {"cheek_roundness": 0.2},
            {"axial_smoothing": math.nan},
        ):
            with self.assertRaises(ValueError):
                validate(bad)

    def test_broad_cheek_support_and_zero_ear_collar(self):
        self.assertEqual(outer_collar(0.5), 1)
        self.assertEqual(outer_collar(0.6), 1)
        self.assertEqual(outer_collar(0.85), 0)
        p = validate({"axial_smoothing": 0.5, "cheek_roundness": 0.1})
        for q in (0, 0.2, 0.5, 0.65, 0.75, 0.85, 0.9):
            a = base_correction(q, 0.4, 0.236, 0.08, -0.12, 0.079, p)
            self.assertEqual(a, base_correction(-q, 0.4, 0.236, 0.08, -0.12, 0.079, p))
            if q >= 0.85:
                self.assertEqual(a, 0)
        # Correction cannot cancel the deliberate center-fold one-sided slope.
        f = lambda q: base_correction(q, 0.4, 0.236, 0.08, -0.12, 0.079, p)
        self.assertLess(abs((f(1e-6) - f(0)) / 1e-6), 1e-7)

    def test_constrained_transverse_has_exact_boundary_jets(self):
        import numpy as np
        from transverse_fit import fit_response, response

        fit = fit_response(0.0001)
        for name, center in (("axial", 1), ("rounding", 0)):
            b = np.asarray(fit[name])
            self.assertEqual(b[0], center)
            self.assertEqual(b[1] - b[0], 0)
            self.assertEqual(b[-1], 0)
            self.assertEqual(b[-1] - b[-2], 0)
            self.assertEqual(b[-1] - 2 * b[-2] + b[-3], 0)
            for q in np.linspace(0, 0.85, 101):
                self.assertLessEqual(abs(response(q, b)), max(abs(b)) + 1e-14)

    def test_offset_budget_is_not_silently_clamped(self):
        from study import require_sample_budget

        require_sample_budget({"within_proposed_2mm_sample_budget": True})
        with self.assertRaises(ValueError):
            require_sample_budget({"within_proposed_2mm_sample_budget": False})

    def test_orbit_gap_is_independently_contracting(self):
        p = validate({"orbit_shallowing": 0.25})
        for gap in (0.00374, 0.00726):
            raw, rim = orbital_support(0.02, 0.02 - gap, 0.0003, p)
            self.assertAlmostEqual(raw - rim, 0.75 * gap, places=14)
        neutral = validate({})
        raw, rim = orbital_support(0.02, 0.015, 0.001, neutral)
        self.assertAlmostEqual(raw - rim, 0.005, places=14)


if __name__ == "__main__":
    unittest.main()
