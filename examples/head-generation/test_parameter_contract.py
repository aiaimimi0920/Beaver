"""Check direct and conditional calibration parameter consumers."""

import ast
import json
import unittest
from pathlib import Path


class ParameterContractTests(unittest.TestCase):
    def test_calibration_contract_matches_runtime_consumers(self):
        root = Path(__file__).parent
        config = json.loads((root / "face_style.json").read_text())["calibration"]
        consumed = set()
        for name in [
            "face",
            "shell",
            "ears",
            "mouth",
            "eye_socket",
            "eye_accents",
            "textures",
            "crease_rules",
            "pipeline",
        ]:
            tree = ast.parse((root / (name + ".py")).read_text())
            bindings = {}
            for assignment in ast.walk(tree):
                if isinstance(assignment, ast.Assign):
                    for target in assignment.targets:
                        if isinstance(target, ast.Name):
                            bindings.setdefault(target.id, []).append(assignment.value)
            for node in ast.walk(tree):
                if (
                    isinstance(node, ast.Subscript)
                    and isinstance(node.value, ast.Name)
                    and node.value.id in ["C", "config"]
                ):
                    expressions = [node.slice]
                    if isinstance(node.slice, ast.Name):
                        self.assertIn(node.slice.id, bindings, "Unresolved dynamic parameter key")
                        expressions = bindings[node.slice.id]
                    found = set()
                    for expression in expressions:
                        for key in ast.walk(expression):
                            if isinstance(key, ast.Constant) and isinstance(key.value, str):
                                found.add(key.value)
                    self.assertTrue(found, "Runtime parameter key must be auditable")
                    consumed.update(found)
        self.assertEqual(set(config) - consumed, set(), "Inactive advertised parameters")
        self.assertEqual(consumed - set(config), set(), "Missing runtime parameters")
        self.assertIn("upper_lip_edge_depth_delta", consumed)
        self.assertIn("lower_lip_edge_depth_delta", consumed)
