"""Exercise the real semantic insertion text against new nose attachments."""

import ast
import json
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from nose_source import adapt_pipeline


class NosePackagingTests(unittest.TestCase):
    def test_semantic_insertion_keeps_old_ordinals_and_adds_new(self):
        tree = ast.parse((HERE.parent / "semantic_eyes/bundle.py").read_text())
        replacements = []
        for node in ast.walk(tree):
            if (
                not isinstance(node, ast.Call)
                or not isinstance(node.func, ast.Name)
                or node.func.id != "replace_once"
                or len(node.args) != 3
            ):
                continue
            old = node.args[1]
            if (
                isinstance(old, ast.Constant)
                and old.value == "PARTS[:] = stable_part_order(PARTS)"
            ):
                replacements.append(
                    eval(
                        compile(
                            ast.Expression(node.args[2]),
                            "actual semantic replacement",
                            "eval",
                        ),
                        {"p": {}},
                    )
                )
        self.assertEqual(len(replacements), 1)
        parts = [
            types.SimpleNamespace(name=n)
            for n in ["skin", "ear47", "ear48", "side_leaf"]
        ]
        original = list(parts)

        def select(items, p):
            items.pop(1)
            return {"selected": len(items)}

        def build(d, shell):
            parts.append(types.SimpleNamespace(name="new nasal part"))

        source = adapt_pipeline(
            'PARTS[:] = stable_part_order(PARTS)\ndef check(data):\n    if True:\n        report["ear_shared_seams"] = verify_ear_seams(data, report["ear_root_fit"])\n',
            {"kind": "pad-muzzle"},
        )
        source = source.replace("PARTS[:] = stable_part_order(PARTS)", replacements[0])
        source += "\nregions=[semantic_part_ordinals[p.name] for p in PARTS]\n"
        ns = {
            "PARTS": parts,
            "shell": None,
            "STYLE": {"nose_design": {}},
            "report": {},
            "stable_part_order": lambda x: x,
        }
        with patch.dict(
            sys.modules,
            {
                "researched_nose_runtime": types.SimpleNamespace(
                    build_attachment=build
                ),
                "semantic_eye_runtime": types.SimpleNamespace(select_components=select),
            },
        ):
            exec(compile(source, "emitted nose/semantic pipeline", "exec"), ns)
        for i, part in enumerate(original):
            self.assertEqual(ns["semantic_part_ordinals"][part.name], 10 + i)
        self.assertEqual(ns["semantic_part_ordinals"]["new nasal part"], 14)
        self.assertEqual(ns["regions"], [10, 12, 13, 14])


if __name__ == "__main__":
    unittest.main()
