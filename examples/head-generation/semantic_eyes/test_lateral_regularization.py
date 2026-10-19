"""Geometry weighting preserves endpoints and does not weaken the final bound."""

import ast
import math
from pathlib import Path
import unittest
from parameters import validate
from source_lateral_regularization import adapt_regularization

RECIPE = Path(__file__).parents[1] / "checkpoints/current/lateral_fit.py"


def functions(source):
    nodes = [
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef)
        and n.name in ("continuous_guide_moves", "validate_combined_guide_moves")
    ]
    scope = {"math": math}
    exec(compile(ast.Module(body=nodes, type_ignores=[]), "<guide>", "exec"), scope)
    return scope


class RegularizationTests(unittest.TestCase):
    def test_disabled_source_is_exact_identity(self):
        source = RECIPE.read_text()
        self.assertEqual(adapt_regularization(source, 1), source)

    def test_weight_one_matches_old_geometry(self):
        old = functions(RECIPE.read_text())["continuous_guide_moves"]
        new = functions(adapt_regularization(RECIPE.read_text(), 0.75))[
            "continuous_guide_moves"
        ]
        stations = [0, 0.12, 0.5, 1]
        bases = [(0, 0, 1), (0.001, 0.001, 1.2), (0.002, 0.002, 1.5), (0.004, 0, 2)]
        self.assertEqual(new(stations, bases, regularization=1), old(stations, bases))
        self.assertEqual(new(stations, bases, regularization=0), [(0, 0, 0)] * 4)
        moves = new(stations, bases)
        self.assertEqual(moves[0], (0, 0, 0))
        self.assertEqual(moves[-1], (0, 0, 0))
        self.assertTrue(all(d[2] == 0 for d in moves))

    def test_weighted_target_still_has_same_four_mm_guard(self):
        source = adapt_regularization(RECIPE.read_text(), 0.75)
        f = functions(source)["continuous_guide_moves"]
        stations = [0, 0.5, 1]
        bases = [(0, 0, 0), (0.0041, 0, 1), (0, 0, 2)]
        moves, raw = f(stations, bases, return_raw=True)
        self.assertAlmostEqual(raw, 0.0041)
        self.assertAlmostEqual(math.dist((0, 0, 0), moves[1]), 0.0041 * 0.75)
        with self.assertRaises(AssertionError):
            f(stations, bases, regularization=1)
        with self.assertRaises(AssertionError):
            f(stations, [(0, 0, 0), (0.006, 0, 1), (0, 0, 2)])

    def test_combined_guard_after_rim_adjustment(self):
        source = adapt_regularization(RECIPE.read_text(), 0.75)
        check = functions(source)["validate_combined_guide_moves"]
        self.assertEqual(check([(0, 0, 0), (0.003, 0, 0)]), 0.003)
        for value in (0.004, 0.0041, float("nan")):
            with self.assertRaises(AssertionError):
                check([(value, 0, 0)])
        self.assertLess(
            source.index("guide_moves[i]=(guide_moves[i][0]+shift"),
            source.index("        validate_combined_guide_moves(guide_moves)"),
        )
        self.assertLess(
            source.index("        validate_combined_guide_moves(guide_moves)"),
            source.index("        for ring,delta in zip(rings,guide_moves):"),
        )

    def test_invalid_weights_and_initial_policy_rejected(self):
        for weight in (-1, 1.01, True, float("nan")):
            with self.assertRaises(ValueError):
                adapt_regularization(RECIPE.read_text(), weight)
        with self.assertRaises(ValueError):
            validate({"lateral_guide_regularization": 0.75})
        self.assertEqual(
            validate(
                {"lateral_aperture_follow": 1, "lateral_guide_regularization": 0.75}
            )["lateral_guide_regularization"],
            0.75,
        )
