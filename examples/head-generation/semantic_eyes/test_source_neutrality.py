"""Compare adapted source behavior with the immutable checkpoint at neutral settings."""

import ast
import json
import math
from pathlib import Path
import tempfile
import unittest
from bundle import build_request, RECIPE, REQUIRED


def packaged_module(request, name):
    for node in ast.walk(ast.parse(request["code"])):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "compile"
            and len(node.args) >= 2
            and isinstance(node.args[1], ast.Constant)
            and node.args[1].value == name
        ):
            return ast.literal_eval(node.args[0])
    raise AssertionError("Missing packaged module: " + name)


class SourceNeutralityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                p = root / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("CPU-only packaging fixture")
            cls.request = build_request(
                {}, "assets/aster/general_face_eyes_neutral/", root
            )
        cls.style = json.loads((RECIPE / "face_style.json").read_text())

    def iris(self, source):
        node = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "iris_color"
        )
        scope = {name: getattr(math, name) for name in ("sqrt", "atan2", "sin", "exp")}
        scope["STYLE"] = self.style
        exec(
            compile(
                ast.Module(body=[node], type_ignores=[]), "<iris behavior>", "exec"
            ),
            scope,
        )
        return scope["iris_color"]

    def test_neutral_iris_pixels_are_exact(self):
        original = self.iris((RECIPE / "textures.py").read_text())
        adapted = self.iris(packaged_module(self.request, "textures"))
        for y in range(41):
            for x in range(41):
                self.assertEqual(original(x / 40, y / 40), adapted(x / 40, y / 40))

    def lower_offset(self, source):
        build = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "build_eyes"
        )
        node = next(
            n
            for n in ast.walk(build)
            if isinstance(n, ast.AugAssign)
            and isinstance(n.target, ast.Name)
            and n.target.id == "y"
            and "lower_cover" in ast.unparse(n.value)
        )
        return compile(
            ast.Expression(body=node.value), "<actual lower-liner expression>", "eval"
        )

    def test_lower_line_identity_and_explicit_scale(self):
        old = self.lower_offset((RECIPE / "face.py").read_text())
        new = self.lower_offset(packaged_module(self.request, "face"))
        scope = {
            "C": self.style["calibration"],
            "H": self.style["calibration"]["height_m"],
            "STYLE": {},
        }
        for lane in (0, 1):
            for i in range(65):
                scope.update(lane=lane, t=-1 + i / 32)
                self.assertEqual(eval(old, scope), eval(new, scope))
                scope["STYLE"] = {"lower_lash_width_scale": 0.4}
                self.assertAlmostEqual(
                    eval(new, scope), eval(old, scope) * 0.4, places=15
                )
                scope["STYLE"] = {}


if __name__ == "__main__":
    unittest.main()
