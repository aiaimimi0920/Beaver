"""Execute the actual packaged intermediate leaf seating on CPU fixtures."""

import ast
from pathlib import Path
from types import SimpleNamespace as NS
import textwrap
import unittest
from test_lash_root_fit import Vector as BaseVector


class Vector(BaseVector):
    def copy(self):
        return Vector(self)

    def __mul__(self, value):
        return Vector(x * value for x in self)

    def __sub__(self, other):
        return Vector(a - b for a, b in zip(self, other))

    def __iadd__(self, other):
        self[:] = [a + b for a, b in zip(self, other)]
        return self

    x = property(lambda self: self[0])


class UpperLayerSeatingTests(unittest.TestCase):
    def run_seating(self, distance=0.001, missing=False):
        source = ast.parse(Path(__file__).with_name("bundle.py").read_text())
        blocks = [
            n.value
            for n in ast.walk(source)
            if isinstance(n, ast.Constant)
            and isinstance(n.value, str)
            and "parent_tree = BVHTree.FromPolygons" in n.value
        ]
        self.assertEqual(len(blocks), 1)
        parts = []
        for side in (-1, 1):
            for prefix in ("Upper lash ", "Inner forked lash blade "):
                vertices = [
                    NS(co=Vector((side * x, distance, 0))) for x in (0.1, 0.15, 0.2, 0.3)
                ]
                parts.append(
                    NS(
                        name=prefix + str(side),
                        data=NS(vertices=vertices, polygons=[], update=lambda: None),
                    )
                )
        tree = NS(
            find_nearest=lambda p: (
                None if missing else Vector((p[0], 0, p[2])),
                None,
                None,
                distance,
            )
        )
        exec(
            textwrap.dedent(blocks[0]),
            {"PARTS": parts, "BVHTree": NS(FromPolygons=lambda *a: tree)},
        )
        return parts

    def test_both_roots_seat_tips_and_parents_stay(self):
        parts = self.run_seating()
        for obj in parts:
            ys = [v.co[1] for v in obj.data.vertices]
            if obj.name.startswith("Upper lash "):
                self.assertEqual(ys, [0.001] * 4)
            else:
                for value, expected in zip(ys, (0, 0.00015625, 0.0005, 0.001)):
                    self.assertAlmostEqual(value, expected)

    def test_rejects_missing_or_excessive_attachment(self):
        for distance in (0.004, 0.005):
            with self.assertRaises(AssertionError):
                self.run_seating(distance)
        with self.assertRaises(AssertionError):
            self.run_seating(missing=True)
