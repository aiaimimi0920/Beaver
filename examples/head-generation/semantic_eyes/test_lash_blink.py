"""Supplemental lash regions participate in their own side's blink field."""

import copy
import json
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import blink_runtime
import bundle
from test_blink_ink_columns import FakeMeshObject


class LashBlinkTests(unittest.TestCase):
    def test_lateral_closure_has_independent_soft_skin_ratio(self):
        c = json.loads((bundle.RECIPE / "face_style.json").read_text())["calibration"]
        for side in (-1, 1):
            for t in (-1.1, -0.7, 0, 0.7, 1.1):
                x = side * c["height_m"] * (c["eye_center_x"] + c["eye_half_width"] * t)
                y = c["origin_y_m"] + c["height_m"] * c["eye_corner_y"] + 0.004
                skin = blink_runtime.closure_delta(x, y, "skin", c)
                lateral = blink_runtime.closure_delta(x, y, "lateral", c)
                self.assertAlmostEqual(lateral, skin * 0.85)
                if abs(t) >= 1:
                    self.assertEqual(lateral, 0)

    def test_all_lash_regions_move_with_declared_field(self):
        c = json.loads((bundle.RECIPE / "face_style.json").read_text())["calibration"]
        groups = [
            ("Upper swept lash tuft ", "upper"),
            ("Attached outer lash wing ", "upper"),
            ("Inner forked lash blade ", "upper"),
            ("Lower outer lash tuft ", "lower"),
            ("Outer lateral eye liner ", "lateral"),
            ("Outer companion lash ", "lateral"),
            ("Embedded lateral child 0 ", "lateral"),
            ("Embedded lateral child 1 ", "lateral"),
            ("Tapered brow ", None),
        ]
        vertices = []
        polygons = []
        regions = []
        ordinals = {}
        expected = []
        for side in (-1, 1):
            for prefix, kind in groups:
                ordinal = len(ordinals) + 10
                ordinals[prefix + str(side)] = ordinal
                x = (
                    side
                    * c["height_m"]
                    * (c["eye_center_x"] + c["eye_half_width"] * 0.4)
                )
                h = (
                    c["origin_y_m"]
                    + c["height_m"] * (c["eye_corner_y"] + c["eye_corner_slope"] * 0.4)
                    + 0.004
                )
                ids = []
                for dz in (0, 0.001, 0.002):
                    i = len(vertices)
                    ids.append(i)
                    vertices.append(
                        SimpleNamespace(
                            index=i, co=SimpleNamespace(x=x, y=0.02, z=h + dz - 0.021)
                        )
                    )
                    expected.append((side, kind))
                polygons.append(SimpleNamespace(index=len(polygons), vertices=ids))
                regions.append(SimpleNamespace(value=ordinal + 1000))
        obj = FakeMeshObject(vertices, polygons, regions)
        before = copy.deepcopy(vertices)
        with patch.object(
            blink_runtime, "verify_pose_orientation", return_value={"mock": True}
        ):
            blink_runtime.build_blink_shapes(obj, ordinals, c)
        self.assertEqual(vertices, before)
        for side, name in [(-1, "blink.L"), (1, "blink.R")]:
            for i, (owner, kind) in enumerate(expected):
                v = before[i].co
                delta = (
                    0
                    if kind is None
                    else blink_runtime.closure_delta(v.x, v.z + 0.021, kind, c)
                )
                if kind is not None:
                    self.assertNotEqual(delta, 0)
                self.assertAlmostEqual(
                    obj.keys[name].data[i].co.z, v.z + (delta if owner == side else 0)
                )
                self.assertEqual(obj.keys[name].data[i].co.x, v.x)
                self.assertEqual(obj.keys[name].data[i].co.y, v.y)
