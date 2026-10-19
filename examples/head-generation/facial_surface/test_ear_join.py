import copy
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch
from source_ear_join import adapt_upper_join

SOURCE = (
    Path(__file__).resolve().parent.parent / "checkpoints/current/ear_silhouette.py"
).read_text()


class Vec:
    def __init__(self, values):
        self.x, self.y, self.z = values

    def __iter__(self):
        return iter((self.x, self.y, self.z))

    def copy(self):
        return Vec(tuple(self))

    def __sub__(self, other):
        return Vec((self.x - other.x, self.y - other.y, self.z - other.z))

    @property
    def length(self):
        return sum(v * v for v in self) ** 0.5


class Tree:
    def __init__(self, count):
        self.rows = []

    def insert(self, point, index):
        self.rows.append((point, index))

    def balance(self):
        pass

    def find(self, point):
        p, i = min(self.rows, key=lambda row: (row[0] - point).length)
        return p, i, (p - point).length


class Normal:
    def copy(self):
        return self

    def dot(self, other):
        return self.value


class EarJoinTests(unittest.TestCase):
    def run_case(self, limit):
        namespace = {}
        exec(adapt_upper_join(SOURCE, True), namespace)
        vertices = [
            types.SimpleNamespace(index=i, co=Vec((x, 0, 1.6)))
            for i, x in enumerate((-0.1, 0.1))
        ]
        polygons = [
            types.SimpleNamespace(index=i, vertices=[i], normal=Normal(), area=1)
            for i in range(2)
        ]

        def update():
            for p, v in zip(polygons, vertices):
                p.normal.value = 1 if abs(v.co.z - 1.6) < limit else -1

        data = types.SimpleNamespace(
            vertices=vertices, polygons=polygons, update=update
        )
        reports = [
            {
                "root_positions": [[x, 0, 1.621]],
                "lower_bridge_root_position": [x, 0, 1.621],
            }
            for x in (-0.1, 0.1)
        ]
        math = types.ModuleType("mathutils")
        math.Vector = Vec
        tree = types.ModuleType("mathutils.kdtree")
        tree.KDTree = Tree
        with patch.dict(sys.modules, {"mathutils": math, "mathutils.kdtree": tree}):
            if limit == 0:
                original = copy.deepcopy(reports)
                with self.assertRaises(AssertionError):
                    namespace["fit_upper_join"](data, reports)
                self.assertEqual(reports, original)
                self.assertTrue(all(v.co.z == 1.6 for v in vertices))
                return
            result = namespace["fit_upper_join"](data, reports)
        self.assertAlmostEqual(vertices[0].co.z, vertices[1].co.z)
        self.assertAlmostEqual(
            reports[0]["root_positions"][0][2], vertices[0].co.z + 0.021
        )
        return result

    def test_identity(self):
        self.assertEqual(adapt_upper_join(SOURCE), SOURCE)

    def test_full(self):
        self.assertEqual(self.run_case(0.01)["accepted_scale"], 1)

    def test_quarter(self):
        result = self.run_case(0.002)
        self.assertEqual(result["accepted_scale"], 0.25)
        self.assertEqual([r["scale"] for r in result["attempts"]], [1, 0.5, 0.25])
        self.assertAlmostEqual(result["maximum_displacement_m"], 0.0012)

    def test_fail_restores(self):
        self.run_case(0)


if __name__ == "__main__":
    unittest.main()
