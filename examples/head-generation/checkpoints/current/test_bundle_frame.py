"""Native frame sizing and independent module configuration without model execution."""

import ast
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from bundle import MODULES, SOURCE_FILES, build_request


class BundleFrameTests(unittest.TestCase):
    def test_overwrite_request_fits_native_frame_with_envelope_headroom(self):
        request = build_request()
        for output in request["outputs"]:
            output["expectedSha256"] = "a" * 64
        frame = {
            "id": "f" * 64,
            "method": "blender.start",
            "params": dict(request, projectId="f" * 64),
        }
        # Default ASCII escaping is conservative for non-ASCII source. Reserve
        # 64 KiB beyond the complete envelope for actual transport metadata.
        encoded = json.dumps(frame).encode("utf-8")
        self.assertLess(len(encoded), 1024 * 1024 - 64 * 1024)

    def test_initial_style_is_deep_copied_for_every_module(self):
        original = {"nested": {"values": [1, 2]}}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for filename in SOURCE_FILES:
                (root / filename).write_text("")
            (root / "face_style.json").write_text(json.dumps(original))
            for name in MODULES:
                (root / (name + ".py")).write_text(
                    "assert STYLE['nested']['values'] == [1, 2]\n"
                    "STYLE['nested']['values'].append(__name__)\n"
                )
            tree = ast.parse(build_request(root)["code"])
            setup = []
            for node in tree.body:
                if (isinstance(node, ast.Expr) and isinstance(node.value, ast.Call)
                        and isinstance(node.value.func, ast.Attribute)
                        and node.value.func.attr == "write_text"):
                    break
                setup.append(node)
            scope = {"beaver_output": lambda path: path}
            with patch.dict(sys.modules):
                exec(compile(ast.Module(body=setup, type_ignores=[]), "<bundle fixture>", "exec"), scope)
                self.assertEqual(scope["_beaver_bundle_style"], original)
                snapshots = [copy.deepcopy(sys.modules[name].STYLE) for name in MODULES]
                self.assertEqual([value["nested"]["values"] for value in snapshots],
                                 [[1, 2, name] for name in MODULES])


if __name__ == "__main__":
    unittest.main()
