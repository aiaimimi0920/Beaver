"""CPU regression for all root-column vertices and fail-closed attachment limits."""

import ast
from pathlib import Path
from types import SimpleNamespace, ModuleType
import sys
import unittest
from unittest.mock import patch


class Coordinate:
    def __init__(self, x, distance):
        self.x = x
        self.distance = distance

    def copy(self):
        return self


def obj(name, distances):
    vertices = [SimpleNamespace(co=Coordinate(0.02, d)) for d in distances]
    vertices.append(SimpleNamespace(co=Coordinate(0.03, 0.02)))
    return SimpleNamespace(
        name=name,
        data=SimpleNamespace(vertices=vertices, polygons=[]),
    )


class TuftRootGateTests(unittest.TestCase):
    def check_gate(self, distances):
        module = ast.parse(Path(__file__).with_name("runtime.py").read_text())
        function = next(
            n
            for n in module.body
            if isinstance(n, ast.FunctionDef)
            and n.name == "validate_retained_tuft_roots"
        )
        namespace = {}
        exec(
            compile(ast.Module(body=[function], type_ignores=[]), "runtime.py", "exec"),
            namespace,
        )
        bvhtree = ModuleType("mathutils.bvhtree")
        tree = SimpleNamespace(find_nearest=lambda co: (None, None, None, co.distance))
        bvhtree.BVHTree = SimpleNamespace(FromPolygons=lambda *args: tree)
        parts = []
        for side in (-1, 1):
            parts.extend(
                [
                    obj("Upper lash " + str(side), [0.0]),
                    obj("Upper swept lash tuft " + str(side), distances),
                ]
            )
        with patch.dict(sys.modules, {"mathutils.bvhtree": bvhtree}):
            return namespace["validate_retained_tuft_roots"](parts)

    def test_every_root_vertex_is_checked_and_free_tip_is_not(self):
        result = self.check_gate([0.0001, 0.0007])
        for side in ("-1", "1"):
            self.assertEqual(result[side]["sampled_root_vertices"], 2)
            self.assertEqual(result[side]["maximum_root_distance_m"], 0.0007)

    def test_later_root_vertex_at_or_above_limit_rejects(self):
        for distance in (0.0008, 0.001, None, float("nan")):
            with self.subTest(distance=distance), self.assertRaises(AssertionError):
                self.check_gate([0.0001, distance])


if __name__ == "__main__":
    unittest.main()
