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

    def test_zero_sampling_restores_legacy_angled_contour(self):
        from brow_shapes import brow_sampling_divisions
        design = validate(self.presets['angled'])
        self.assertEqual(brow_sampling_divisions(design, {}), 0)
        self.assertEqual(brow_sampling_divisions(design, {'sampling_first_pass': 1}), 32)
        self.assertEqual(brow_sampling_divisions(design, {'sampling_brow_divisions': 16}), 16)
        self.assertEqual(brow_polygons(design, .04, .002), brow_polygons(design, .04, .002, 64))

    def test_explicit_lower_sampling_keeps_angled_peak_and_valid_outline(self):
        design = validate({**self.presets["angled"], "peak": .63})
        original = brow_polygons(design, .04, .002)
        coarse = brow_polygons(design, .04, .002, 16)
        self.assertLess(len(coarse[0]), len(original[0]))
        brow_validate_polygon(coarse[0])
        length = .04 * design['length_scale']
        offset = (.04 - length) * .5
        self.assertTrue(any(abs(x-(offset+length*.63)) < 1e-10 for x,y in coarse[0]))
        with self.assertRaises(ValueError):
            brow_polygons(design, .04, .002, 15)

    def test_sampling_preflight_rejects_unsupported_or_missing_design(self):
        style = copy.deepcopy(self.style)
        style['sampling_brow_divisions'] = 16
        for design in (None, validate(self.presets['curl']), validate(self.presets['symbols'])):
            with self.subTest(design=design), self.assertRaises(ValueError):
                validate_geometry_style(design, style)
        self.assertGreater(validate_geometry_style(validate(self.presets['angled']), style)['local_area_m2'], 0)

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
