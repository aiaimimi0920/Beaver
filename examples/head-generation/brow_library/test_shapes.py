import ast
import copy
import json
import math
from pathlib import Path
import unittest

from brow_design import validate
from brow_shapes import brow_area, brow_polygons, brow_validate_polygon
from brow_source import adapt, retained_sources, validate_geometry_style

HERE = Path(__file__).resolve().parent


class BrowShapeTests(unittest.TestCase):
    def setUp(self):
        self.presets = json.loads((HERE / "presets.json").read_text())
        self.style = json.loads((HERE.parent / "checkpoints/current/face_style.json").read_text())
        self.source = (HERE.parent / "checkpoints/current/face.py").read_text()

    def polygons(self, name, **changes):
        return brow_polygons(validate({**self.presets[name], **changes}), 0.046728, 0.0021063)

    def test_every_researched_preset_has_real_positive_geometry(self):
        self.assertEqual(len(self.presets), 11)
        signatures = set()
        for name, raw in self.presets.items():
            with self.subTest(name=name):
                d = validate(raw)
                polys = self.polygons(name)
                self.assertTrue(all(brow_area(p) > 1e-8 for p in polys))
                self.assertEqual(validate_geometry_style(d, self.style)["islands_per_side"], len(polys))
                signatures.add(json.dumps(polys))
        self.assertEqual(len(signatures), len(self.presets))

    def test_shape_changes_are_not_just_ids(self):
        def extent(polys, axis):
            values = [p[axis] for poly in polys for p in poly]
            return max(values) - min(values)
        self.assertLess(extent(self.polygons("compact-leaf"), 0), extent(self.polygons("linear-fine"), 0) * 0.5)
        self.assertGreater(extent(self.polygons("linear-wedge"), 1), extent(self.polygons("linear-fine"), 1) * 2)
        self.assertGreater(max(y for _, y in self.polygons("arc-fine")[0]), max(y for _, y in self.polygons("linear-fine")[0]))
        symbols = self.polygons("symbols")
        self.assertEqual(len(symbols), 3)
        for left, right in zip(symbols, symbols[1:]):
            self.assertLess(max(x for x, _ in left), min(x for x, _ in right))

    def test_controls_have_observable_geometry(self):
        for name, change in [("angled", {"peak": .4}), ("arc-fine", {"arch_ratio": .1}), ("linear-fine", {"width_scale": .9}), ("multipoint", {"detail_count": 4}), ("curl", {"curl_turns": .8})]:
            self.assertNotEqual(self.polygons(name), self.polygons(name, **change))

    def test_reject_invalid_and_self_intersecting_geometry(self):
        for bad in [True, float("nan"), float("inf"), -1]:
            with self.assertRaises(ValueError):
                validate({"kind": "arc", "schema_version": 1, "width_scale": bad})
        for extra in [{"kind": "no"}, {"detail_count": 3.5}, {"schema_version": True}, {"extra": 1}, {"clearance_m": .0003, "thickness_m": .0004}]:
            with self.assertRaises(ValueError):
                validate({"kind": "arc", "schema_version": 1, **extra})
        with self.assertRaises(ValueError):
            brow_validate_polygon([(0, 0), (1, 1), (0, 1), (1, 0)])
        with self.assertRaises(ValueError):
            self.polygons("curl", length_scale=.12, width_scale=3)

    def test_legacy_source_identity_and_other_functions_preserved(self):
        self.assertEqual(adapt(self.source, None), self.source)
        changed = adapt(self.source, validate(self.presets["curl"]))
        original = {n.name: ast.dump(n) for n in ast.parse(self.source).body if isinstance(n, ast.FunctionDef)}
        replacement = {n.name: ast.dump(n) for n in ast.parse(changed).body if isinstance(n, ast.FunctionDef)}
        for name in original.keys() - {"build_eyes"}:
            self.assertEqual(original[name], replacement[name], name)
        self.assertIn("build_designed_brow(side", changed)
        self.assertEqual(set(retained_sources()), {"brow_design.py", "brow_shapes.py", "brow_mesh.py", "brow_source.py"})

    def test_adapter_rejects_contract_drift(self):
        with self.assertRaises(ValueError):
            adapt(self.source.replace("grid_faces(2, 25, side < 0)", "grid_faces(2, 26, side < 0)"), validate(self.presets["arc-fine"]))


if __name__ == "__main__":
    unittest.main()
