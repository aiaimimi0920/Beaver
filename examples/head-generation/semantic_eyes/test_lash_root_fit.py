"""Exercise the packaged rigid root fit, including a misleading centroid."""

import ast
import math
from pathlib import Path
from types import SimpleNamespace
import textwrap
import unittest


class Vector(list):
    def copy(self):
        return Vector(self)

    def __add__(self, other):
        return Vector(a + b for a, b in zip(self, other))

    def __sub__(self, other):
        return Vector(a - b for a, b in zip(self, other))

    @property
    def length(self):
        return math.sqrt(sum(v * v for v in self))


class Strip:
    def find_nearest(self, point):
        nearest = Vector((max(-0.0005, min(0.0005, point[0])), 0, point[2]))
        return nearest, None, None, (nearest - point).length


class LashRootFitTests(unittest.TestCase):
    def fit(self, roots, center, target):
        tree = ast.parse(Path(__file__).with_name("bundle.py").read_text())
        blocks = [
            node.value
            for node in ast.walk(tree)
            if isinstance(node, ast.Constant)
            and isinstance(node.value, str)
            and node.value.startswith("        delta = target-center\n")
            and "def score(move):" in node.value
        ]
        self.assertEqual(len(blocks), 1)
        scope = dict(roots=roots, center=center, target=target, tree=Strip())
        exec(textwrap.dedent(blocks[0]), scope)
        return scope["delta"]

    def test_centroid_contact_does_not_hide_disconnected_edges(self):
        roots = [
            SimpleNamespace(co=Vector((x * 0.001, 0, 0)))
            for x in (-0.2, -0.1, 0, 0.2, 1.2, 1.3)
        ]
        center = Vector((0.0004, 0, 0))
        target = center.copy()
        tree = Strip()
        before = max(tree.find_nearest(v.co)[3] for v in roots)
        delta = self.fit(roots, center, target)
        after = max(tree.find_nearest(v.co + delta)[3] for v in roots)
        self.assertGreater(before, 0.00079)
        self.assertLess(after, before * 0.5)
        self.assertLess(delta.length, 0.004)
        self.assertEqual(delta[2], 0)
        for a, b in zip(roots, roots[1:]):
            self.assertAlmostEqual(
                ((a.co + delta) - (b.co + delta)).length, (a.co - b.co).length
            )

    def test_fully_seated_section_stays_put(self):
        roots = [SimpleNamespace(co=Vector((x, 0, 0))) for x in (-0.0001, 0.0001)]
        self.assertEqual(
            self.fit(roots, Vector((0, 0, 0)), Vector((0, 0, 0))), [0, 0, 0]
        )
