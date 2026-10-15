"""Verify extraction contracts and the real bundle import boundary without Blender."""

import ast
import hashlib
import json
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from ast_contract import canonical_function_dump
from bundle import MODULES, SOURCE_FILES, OUTPUT_PREFIX, build_request

ROOT = Path(__file__).parent
CONTRACT = json.loads((ROOT / "accent-attachment-refactor-contract.json").read_text())


def combined_accent_tree(tree=None):
    """Reassemble the historical AST for existing cross-version scope checks."""
    tree = tree if tree is not None else ast.parse((ROOT / "eye_accents.py").read_text())
    attachment = ast.parse((ROOT / "accent_attachment.py").read_text())
    functions = {
        node.name: node for node in tree.body + attachment.body
        if isinstance(node, ast.FunctionDef)
    }
    preamble = [
        node for node in tree.body if not isinstance(node, ast.FunctionDef)
        and not (isinstance(node, ast.ImportFrom) and node.module == "accent_attachment")
    ]
    tree.body = preamble + [functions[name] for name in CONTRACT["function_order"]]
    return tree


class AccentAttachmentRefactorTests(unittest.TestCase):
    def test_all_original_functions_preserve_their_exact_ast(self):
        functions = []
        for name in ("eye_accents.py", "accent_attachment.py"):
            functions.extend(node for node in ast.parse((ROOT / name).read_text()).body
                             if isinstance(node, ast.FunctionDef))
        self.assertEqual(len(functions), len(CONTRACT["canonical_sha256"]))
        actual = {
            node.name: hashlib.sha256(canonical_function_dump(
                ast.dump(node, include_attributes=False)).encode()).hexdigest()
            for node in functions
        }
        self.assertEqual(actual, CONTRACT["canonical_sha256"])

    def test_every_declared_python_source_fits_without_an_exception(self):
        # A conservative upper bound: even docstrings count here, unlike the
        # production checker. No generated source needs a path-specific waiver.
        for name in SOURCE_FILES:
            if name.endswith(".py"):
                lines = (ROOT / name).read_text().splitlines()
                effective = sum(bool(line.strip()) and not line.lstrip().startswith("#")
                                for line in lines)
                self.assertLessEqual(effective, 500, name)

    def test_bundle_declares_and_loads_attachment_before_its_consumer(self):
        self.assertLess(MODULES.index("face"), MODULES.index("accent_attachment"))
        self.assertLess(MODULES.index("accent_attachment"), MODULES.index("eye_accents"))
        request = build_request()
        self.assertIn(OUTPUT_PREFIX + "accent_attachment.py",
                      [output["path"] for output in request["outputs"]])
        self.assertIn((ROOT / "accent_attachment.py").read_text(),
                      [node.value for node in ast.walk(ast.parse(request["code"]))
                       if isinstance(node, ast.Constant) and isinstance(node.value, str)])

    def test_imports_share_state_and_rebuild_resets_support_on_failure(self):
        face = types.ModuleType("face")
        face.STYLE, face.PARTS = {}, []
        modules = {"face": face}
        eye_tree = ast.parse((ROOT / "eye_accents.py").read_text())
        for node in eye_tree.body:
            if isinstance(node, ast.ImportFrom) and node.module not in ("face", "accent_attachment"):
                stub = types.ModuleType(node.module)
                for alias in node.names:
                    setattr(stub, alias.name, lambda *args: None)
                modules[node.module] = stub
        with patch.dict(sys.modules, modules):
            for name in ("accent_attachment", "eye_accents"):
                module = types.ModuleType(name)
                module.STYLE = face.STYLE
                sys.modules[name] = module
                exec(compile((ROOT / (name + ".py")).read_text(), name, "exec"), module.__dict__)
            eye, attachment = sys.modules["eye_accents"], sys.modules["accent_attachment"]
            self.assertIs(eye.STYLE, attachment.STYLE)
            self.assertIs(eye.PARTS, attachment.PARTS)
            for name in CONTRACT["attachment_functions"]:
                self.assertIs(getattr(eye, name), getattr(attachment, name))
            support = object()

            def failing_builder(side):
                self.assertIs(eye._EVALUATED_LID_SUPPORT, support)
                raise RuntimeError("fixture rebuild failure")

            eye.build_upper_accents = failing_builder
            with self.assertRaisesRegex(RuntimeError, "fixture rebuild failure"):
                eye.rebuild_accents_on_evaluated_support(support)
            self.assertIsNone(eye._EVALUATED_LID_SUPPORT)


if __name__ == "__main__":
    unittest.main()
