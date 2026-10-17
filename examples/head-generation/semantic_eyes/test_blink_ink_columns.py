"""CPU-only blink membership/column contracts; no native topology claim."""

import ast
import copy
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import blink_runtime
import bundle


class FakeMeshObject:
    def __init__(self, vertices, polygons, regions):
        self.data = SimpleNamespace(
            vertices=vertices,
            polygons=polygons,
            attributes={"NormalRegion": SimpleNamespace(data=regions)},
        )
        self.keys = {}

    def shape_key_add(self, *, name):
        key = SimpleNamespace(data=copy.deepcopy(self.data.vertices), value=0.0)
        self.keys[name] = key
        return key


class BlinkInkColumnTests(unittest.TestCase):
    def setUp(self):
        self.c = json.loads((bundle.RECIPE / "face_style.json").read_text())[
            "calibration"
        ]

    def seam(self, t):
        c = self.c
        return (
            c["origin_y_m"]
            + c["height_m"] * (c["eye_corner_y"] + c["eye_corner_slope"] * t)
            - 0.0015 * (1 - t * t)
        )

    def fixture(self):
        vertices, polygons, regions, columns = [], [], [], []

        def vertex(x, y, z):
            index = len(vertices)
            vertices.append(
                SimpleNamespace(index=index, co=SimpleNamespace(x=x, y=y, z=z))
            )
            return index

        ordinals = {"Upper lash -1": 17, "Upper lash 1": 18}
        for side, ordinal in ((-1, 17), (1, 18)):
            for zone, t in enumerate((-0.8, -0.2, 0.4, 0.9)):
                x = (
                    side
                    * self.c["height_m"]
                    * (self.c["eye_center_x"] + self.c["eye_half_width"] * t)
                )
                # Unequal station and side floors detect accidental global/abs(x) grouping.
                floor = self.seam(t) + 0.004 + zone * 0.001 + (0.002 if side > 0 else 0)
                ids = [
                    vertex(x, depth, floor + rise - 0.021)
                    for depth, rise in ((0.02, 0.0), (0.021, 0.001), (0.02, 0.003))
                ]
                polygons.append(SimpleNamespace(index=len(polygons), vertices=ids))
                regions.append(SimpleNamespace(value=ordinal + zone * 1000))
                columns.append((side, t, floor, ids))
                # Unowned geometry at the same x must not lower the ink floor or move.
                vertex(x, 0.03, floor - 0.04)
        return FakeMeshObject(vertices, polygons, regions), ordinals, columns

    def run_shapes(self, obj, ordinals, **kwargs):
        with patch.object(
            blink_runtime, "verify_pose_orientation", return_value={"mock": True}
        ) as gate:
            report = blink_runtime.build_blink_shapes(obj, ordinals, self.c, **kwargs)
        gate.assert_called_once_with(obj)
        self.assertEqual(report["continuous_orientation"], {"mock": True})
        return report

    def test_tagged_columns_land_on_seam_without_changing_neutral(self):
        obj, ordinals, columns = self.fixture()
        before = copy.deepcopy(obj.data.vertices)
        report = self.run_shapes(obj, ordinals, ink_floor_closure=True)
        self.assertEqual(obj.data.vertices, before)
        owned = {i for _, _, _, ids in columns for i in ids}
        for side, name in ((-1, "blink.L"), (1, "blink.R")):
            key = obj.keys[name]
            self.assertEqual(key.value, 0.0)
            self.assertEqual(report[name]["moved_vertices"], 12)
            for column_side, t, floor, ids in columns:
                delta = self.seam(t) - floor if side == column_side else 0.0
                for i in ids:
                    self.assertAlmostEqual(key.data[i].co.z, before[i].co.z + delta)
                    self.assertEqual(key.data[i].co.x, before[i].co.x)
                    self.assertEqual(key.data[i].co.y, before[i].co.y)
                self.assertAlmostEqual(
                    key.data[ids[-1]].co.z - key.data[ids[0]].co.z, 0.003
                )
                if side == column_side:
                    self.assertAlmostEqual(key.data[ids[0]].co.z + 0.021, self.seam(t))
            for i in set(range(len(before))) - owned:
                self.assertEqual(key.data[i].co, before[i].co)

    def test_false_and_default_preserve_analytic_upper_motion(self):
        results = []
        for kwargs in ({}, {"ink_floor_closure": False}):
            obj, ordinals, columns = self.fixture()
            before = copy.deepcopy(obj.data.vertices)
            self.run_shapes(obj, ordinals, **kwargs)
            self.assertEqual(obj.data.vertices, before)
            for column_side, _, _, ids in columns:
                for side, name in ((-1, "blink.L"), (1, "blink.R")):
                    for i in ids:
                        v = before[i].co
                        delta = blink_runtime.closure_delta(
                            v.x, v.z + 0.021, "upper", self.c
                        )
                        self.assertAlmostEqual(
                            obj.keys[name].data[i].co.z,
                            v.z + (delta if side == column_side else 0),
                        )
            results.append(obj.keys)
        self.assertEqual(results[0], results[1])

    def test_packaging_gate_requires_illustrated_without_tuft(self):
        original_adapt = bundle.adapt_style
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in bundle.REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"CPU fixture")
            for illustrated in (False, True):
                for tuft in (0, 1):
                    with self.subTest(illustrated=illustrated, tuft=tuft):

                        def adapt(style, parameters):
                            return original_adapt(
                                dict(style, illustrated_eye_art=illustrated), parameters
                            )

                        with patch.object(bundle, "adapt_style", side_effect=adapt):
                            request = bundle.build_request(
                                dict(
                                    supplemental_lashes=0,
                                    lid_fold=0,
                                    lash_contour_follow=1,
                                    short_upper_tuft=tuft,
                                ),
                                "assets/aster/general_face_eyes_ink_cpu/",
                                root,
                                blink=True,
                            )
                        calls = []
                        source = request["code"]
                        trees = [ast.parse(source)]
                        # The generated pipeline is also retained as a source string.
                        for node in ast.walk(trees[0]):
                            if (
                                isinstance(node, ast.Constant)
                                and isinstance(node.value, str)
                                and 'report["eyelid_closure"] = build_blink_shapes'
                                in node.value
                            ):
                                trees.append(ast.parse(node.value))
                        for tree in trees:
                            calls.extend(
                                n
                                for n in ast.walk(tree)
                                if isinstance(n, ast.Call)
                                and isinstance(n.func, ast.Name)
                                and n.func.id == "build_blink_shapes"
                            )
                        self.assertTrue(calls)
                        for call in calls:
                            flags = [
                                ast.literal_eval(k.value)
                                for k in call.keywords
                                if k.arg == "ink_floor_closure"
                            ]
                            self.assertEqual(flags, [illustrated and not tuft])


if __name__ == "__main__":
    unittest.main()
