"""Regression of repair eligibility, not permission to waive final mesh checks."""

import ast
from pathlib import Path
import unittest
from source_optional_bridges import adapt_bridge_pair_source, adapt_ear_boundary_source

RECIPE = Path(__file__).resolve().parents[1] / "checkpoints/current"


class OptionalBridgeTests(unittest.TestCase):
    def setUp(self):
        scope = {}
        exec(adapt_bridge_pair_source((RECIPE / "bridge_pairs.py").read_text()), scope)
        self.pair = scope["paired_bridge_indices"]
        self.points = [
            (-1, 0, 0),
            (-1, 0.1, 0),
            (-1, 0, 0.1),
            (1, 0, 0),
            (1, 0.1, 0),
            (1, 0, 0.1),
        ]
        self.candidates = {10: [0, 1, 2], 20: [3, 4, 5]}

    def test_no_invalid_bridge_needs_no_pair(self):
        self.assertEqual(self.pair(self.points, self.candidates, []), [])
        scope = {}
        exec(adapt_ear_boundary_source((RECIPE / "ear_boundary.py").read_text()), scope)
        moved = [(x, y + 0.001, z) for x, y, z in self.points]
        self.assertEqual(
            scope["coherent_bridge_targets"](self.points, moved, []), moved
        )
        self.assertEqual(
            scope["blend_rim_displacements"](moved, moved, {}, set(), set())[0], moved
        )

    def test_required_pair_stays_strict(self):
        self.assertEqual(self.pair(self.points, self.candidates, [10]), [10, 20])
        self.assertEqual(self.pair(self.points, self.candidates, [10, 20]), [10, 20])
        for candidates, initial in [
            ({10: [0, 1, 2]}, [10]),
            (self.candidates, [10, 20, 30]),
            ({**self.candidates, 30: [3, 4, 5]}, [10]),
        ]:
            with self.assertRaises(AssertionError):
                self.pair(self.points, candidates, initial)
        self.points[3] = (1.01, 0, 0)
        with self.assertRaises(AssertionError):
            self.pair(self.points, self.candidates, [10, 20])

    def test_other_geometry_assertions_are_unchanged(self):
        old = (RECIPE / "ear_boundary.py").read_text()
        new = adapt_ear_boundary_source(old)

        def checks(source):
            return [
                ast.dump(n, include_attributes=False)
                for n in ast.walk(ast.parse(source))
                if isinstance(n, ast.Assert)
                and "bridge_groups" not in ast.unparse(n.test)
            ]

        self.assertEqual(checks(old), checks(new))
        # The scope restriction mentions bridge_ids, so remains among these checks.
        self.assertIn("Bridge outside authorized local support", new)
        self.assertIn("Ear-jaw rim fairing inverted a face", new)

    def test_contract_drift_is_rejected(self):
        with self.assertRaises(ValueError):
            adapt_bridge_pair_source("")
        with self.assertRaises(ValueError):
            adapt_ear_boundary_source("")


if __name__ == "__main__":
    unittest.main()
