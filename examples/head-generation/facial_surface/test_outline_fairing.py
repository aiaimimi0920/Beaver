import ast
import math
from pathlib import Path
import unittest
from source_outline_guard import guard_outline, outline_turns
from source_outline_trace import instrument_outline
from source_outline_fairing import adapt_outline_fairing


class OutlineFairingTests(unittest.TestCase):
    def test_identity_and_budget(self):
        original = [(0, 0, 0), (2, 0, 1), (3, 0, 2), (3.5, 0, 3)]
        result, report = guard_outline(original, original, [0, 1, 2, 3], original)
        self.assertEqual(result, original)
        self.assertEqual(report["outline_backtracking_factor"], 1)
        bad = [*original[:2], (3, 0, 2.8), original[3]]
        result, report = guard_outline(original, bad, [0, 1, 2, 3], original)
        self.assertLess(report["outline_backtracking_factor"], 1)
        self.assertTrue(all(t <= 1e-9 for t in outline_turns(result, [0, 1, 2, 3])))
        self.assertLessEqual(report["maximum_displacement_m"], 0.8 + 1e-9)
        self.assertTrue(report["rejected_outline_steps"])

    def test_no_new_turn_from_zero_fallback(self):
        original = [(0, 0, 0), (1, 0, 1), (2, 0, 2)]
        bad = [(0, 0, 0), (1, 0, 1.4), (2, 0, 2)]
        result, report = guard_outline(original, bad, [0, 1, 2], original)
        self.assertEqual(result, original)
        self.assertEqual(report["outline_fairing_status"], "skipped_no_safe_shape_step")

    def test_invalid_chain_and_nonfinite_rejected(self):
        for points in [
            [(0, 0, 0), (1, 0, 0), (2, 0, 1)],
            [(0, 0, 0), (1, 0, 1), (math.nan, 0, 2)],
        ]:
            with self.assertRaises(ValueError):
                outline_turns(points, [0, 1, 2])

    def test_strict_source_and_existing_limits(self):
        src = (
            Path(__file__).parent.parent / "checkpoints/current/shell.py"
        ).read_text()
        result = adapt_outline_fairing(instrument_outline(src))
        ast.parse(result)
        self.assertIn("assert maximum < 0.02 * height", result)
        self.assertIn("final_boundary_config = dict(fair_config)", result)
        with self.assertRaises(ValueError):
            adapt_outline_fairing(result)
