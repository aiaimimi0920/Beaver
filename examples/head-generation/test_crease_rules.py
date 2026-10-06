import json
from pathlib import Path
import unittest
from crease_rules import regional_weight, semantic_weight

C = json.loads((Path(__file__).parent / "face_style.json").read_text())["calibration"]


class CreaseRulesTests(unittest.TestCase):
    def test_profiles_are_bounded_and_tapered(self):
        for key in ["face_center_crease_profile", "face_side_crease_profile"]:
            for i in range(-10, 111):
                self.assertTrue(0 <= regional_weight(i / 100, C[key]) <= 1)
            self.assertEqual(regional_weight(-0.1, C[key]), 0)
            self.assertEqual(regional_weight(1.1, C[key]), 0)

    def test_center_and_side_use_distinct_semantics(self):
        role, value = semantic_weight((0, 0, 0.25), (0, 0, 0.3), 1, 1, 1, 0, C)
        self.assertEqual(role, 1)
        self.assertGreater(value, 0.9)
        role, value = semantic_weight((0.78, 0, 0.25), (0.78, 0, 0.3), 1, 1, 1, 0, C)
        self.assertEqual(role, 2)
        self.assertGreater(value, 0)

    def test_crosswise_edges_and_other_chains_are_excluded(self):
        self.assertEqual(semantic_weight((0.78, 0, 0.3), (0.7, 0, 0.3), 1, 1, 1, 0, C), (0, 0))
        self.assertEqual(semantic_weight((0.5, 0, 0.25), (0.5, 0, 0.3), 1, 1, 1, 0, C), (0, 0))


if __name__ == "__main__":
    unittest.main()
