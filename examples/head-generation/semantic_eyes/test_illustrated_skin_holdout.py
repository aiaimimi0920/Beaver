"""CPU mask contracts only; filtered native outline appearance remains unverified."""

import ast
import copy
import json
import math
from pathlib import Path
import unittest

from illustrated_art import SKIN_HOLDOUT, adapt_sources
from parameters import adapt_style


class IllustratedSkinHoldoutTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).parents[1] / "checkpoints/current"
        base = json.loads((self.root / "face_style.json").read_text())
        self.c = adapt_style(
            base,
            dict(
                upper_arch=1.4,
                upper_roundness=1.4,
                upper_horizontal_roundness=0.72,
                corner_slope_offset=-0.014,
            ),
        )["calibration"]
        self.ns = dict(
            sqrt=math.sqrt,
            H=self.c["height_m"],
            Y0=self.c["origin_y_m"],
            C=self.c,
            STYLE={"ocular_skin_outline_holdout": True},
        )
        exec(SKIN_HOLDOUT, self.ns)
        self.mask = self.ns["ocular_skin_holdout"]

    @staticmethod
    def uv(x, y):
        return 0.02 + 0.53 * (0.5 + x / 0.235), 0.02 + 0.96 * (y - 1.5) / 0.31

    def test_calibrated_plateau_transition_and_symmetry(self):
        c = self.c
        h = c["height_m"]
        center_x = h * c["eye_center_x"]
        center_y = c["origin_y_m"] + h * c["eye_corner_y"]
        rx = 1.08 * h * c["eye_half_width"]
        for q, expected in ((0, 1), (0.99, 1), (1.04, 0.5), (1.081, 0)):
            for side in (-1, 1):
                self.assertAlmostEqual(
                    self.mask(*self.uv(side * (center_x + rx * q), center_y)), expected
                )
        for upper in (False, True):
            arch = c["eye_upper_arch"] if upper else c["eye_lower_arch"]
            for t in (-1, -0.9375, -0.5, 0, 0.5, 0.9375, 1):
                power = c["eye_upper_power"] if upper else c["eye_lower_power"]
                horizontal = (
                    c["eye_horizontal_power"]
                    if upper
                    else c["eye_lower_horizontal_power"]
                )
                y = (
                    center_y
                    + (1 if upper else -1)
                    * h
                    * arch
                    * max(0, 1 - abs(t) ** horizontal) ** power
                )
                x = h * (c["eye_center_x"] + c["eye_half_width"] * t)
                self.assertEqual(self.mask(*self.uv(x, y)), 1)
                self.assertEqual(self.mask(*self.uv(-x, y)), 1)

    def test_medial_expansion_preserves_temporal_extent_and_symmetry(self):
        c = self.c
        h = c["height_m"]
        cx = h * c["eye_center_x"]
        cy = c["origin_y_m"] + h * c["eye_corner_y"]
        for direction, scale in ((-1, 1.40), (1, 1.08)):
            for q, expected in ((0.99, 1.0), (1.04, 0.5), (1.081, 0.0)):
                x = cx + direction * scale * h * c["eye_half_width"] * q
                for side in (-1, 1):
                    self.assertAlmostEqual(self.mask(*self.uv(side * x, cy)), expected)

    def test_supported_width_spacing_extrema_keep_nasal_center_clear(self):
        base = json.loads((self.root / "face_style.json").read_text())
        for spacing in (0.94, 1.04):
            for width in (0.92, 1.08):
                c = adapt_style(base, {"eye_spacing": spacing, "eye_width": width})[
                    "calibration"
                ]
                self.ns["C"] = c
                h, y0 = self.ns["H"], self.ns["Y0"]
                cy = y0 + h * c["eye_corner_y"]
                self.assertEqual(self.mask(*self.uv(0, cy)), 0.0)
                for side in (-1, 1):
                    x = side * h * (c["eye_center_x"] - c["eye_half_width"])
                    y = cy - h * c["eye_corner_slope"]
                    self.assertEqual(self.mask(*self.uv(x, y)), 1.0)

    def test_disabled_mask_preserves_entire_legacy_control_map(self):
        self.ns["STYLE"].clear()
        pixel, legacy = self.pixel_function(), self.pixel_function(legacy=True)
        for py in range(256):
            for px in range(512):
                u = (px + 0.5) / 512
                self.assertEqual(pixel(u, py), legacy(u, py))

    def pixel_function(self, legacy=False):
        sources = adapt_sources(
            {
                name: (self.root / name).read_text()
                for name in ("textures.py", "pipeline.py", "eye_socket.py")
            }
        )
        tree = ast.parse(sources["textures.py"])
        wanted = {
            "flat_region",
            "slot",
            "iris_island",
            "ear_island",
            "sclera_island",
            "ink_island",
            "holdout",
        }
        bake = next(
            n
            for n in tree.body
            if isinstance(n, ast.FunctionDef) and n.name == "bake_control_maps"
        )
        statements = []
        for node in ast.walk(bake):
            if (
                isinstance(node, ast.Assign)
                and isinstance(node.targets[0], ast.Name)
                and node.targets[0].id in wanted
            ):
                statements.append(copy.deepcopy(node))
            elif (
                isinstance(node, ast.Expr)
                and isinstance(node.value, ast.Call)
                and isinstance(node.value.func, ast.Attribute)
                and isinstance(node.value.func.value, ast.Name)
                and node.value.func.value.id == "makeup"
            ):
                result = copy.deepcopy(node.value.args[0])
                if legacy:
                    result.elts[2] = ast.parse("1 if holdout else 0", mode="eval").body
                statements.append(ast.Return(value=result))
        function = ast.parse("def pixel(u, py):\n    pass\n").body[0]
        function.body = statements
        ns = dict(self.ns, colors=[0, 1], lip=[False, True])
        exec(
            compile(
                ast.fix_missing_locations(ast.Module(body=[function], type_ignores=[])),
                "control-map-contract",
                "exec",
            ),
            ns,
        )
        return ns["pixel"]

    def test_actual_512x256_map_preserves_other_channels_and_islands(self):
        pixel, legacy = self.pixel_function(), self.pixel_function(legacy=True)
        c = self.c
        h = c["height_m"]
        cx, cy = h * c["eye_center_x"], c["origin_y_m"] + h * c["eye_corner_y"]
        reach = 1.08 * 1.40 * h * c["eye_half_width"]
        lower = cy - 1.08 * (h * c["eye_lower_arch"] + 0.002)
        upper = cy + 1.08 * (h * c["eye_upper_arch"] + 0.002)
        changed = plateau = transition = 0
        for py in range(256):
            v = (py + 0.5) / 256
            for px in range(512):
                u = (px + 0.5) / 512
                before, after = legacy(u, py), pixel(u, py)
                self.assertEqual(before[:2] + before[3:], after[:2] + after[3:])
                self.assertTrue(math.isfinite(after[2]) and 0 <= after[2] <= 1)
                if not (0.02 < u < 0.55 and 0.02 < v < 0.98):
                    self.assertEqual(
                        before, after
                    )  # Includes ear and all prior islands.
                if after != before:
                    changed += 1
                    x = ((u - 0.02) / 0.53 - 0.5) * 0.235
                    y = 1.5 + (v - 0.02) / 0.96 * 0.31
                    self.assertTrue(cx - reach < abs(x) < cx + reach)
                    self.assertTrue(lower < y < upper)
                    self.assertGreater(
                        abs(x), cx - reach
                    )  # Nasal core stays outside expanded medial support.
                    plateau += after[2] == 1
                    transition += 0 < after[2] < 1
                if 0.02 < u < 0.55:
                    x = ((u - 0.02) / 0.53 - 0.5) * 0.235
                    if abs(x) <= cx - reach or abs(x) >= 0.083:
                        self.assertEqual(before, after)
        self.assertGreater(changed, 0)
        self.assertGreater(plateau, 0)
        self.assertGreater(transition, 0)


if __name__ == "__main__":
    unittest.main()
