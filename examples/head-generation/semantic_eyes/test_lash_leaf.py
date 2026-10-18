"""The packaged narrowing preserves depth, lower attachment and unrelated objects."""

import ast
from pathlib import Path
from types import SimpleNamespace as NS
import textwrap
import unittest


class LeafNarrowingTests(unittest.TestCase):
    def test_column_narrowing_is_symmetric_and_root_preserving(self):
        for lift_scale in (None, 0.0, 0.5, 1.0):
            with self.subTest(lift_scale=lift_scale):
                self.check_columns(lift_scale)

    def check_columns(self, lift_scale):
        tree = ast.parse(Path(__file__).with_name("bundle.py").read_text())
        block = next(
            node.value
            for node in ast.walk(tree)
            if isinstance(node, ast.Constant)
            and isinstance(node.value, str)
            and node.value.startswith("\n        for wing in PARTS:")
        )
        parts = []
        records = []
        for side, prefix in [
            (-1, "Attached outer lash wing "),
            (1, "Attached outer lash wing "),
            (1, "Upper lash "),
        ]:
            vertices = []
            for t in (0.6, 0.65, 0.75, 0.85, 1.0, 1.05, 1.1):
                for offset in (-0.002, 0.002):
                    v = NS(co=NS(x=side * (0.04 + 0.02 * t), y=-0.03, z=1.6 + offset))
                    vertices.append(v)
                    records.append((v, prefix, t, side, offset))
            parts.append(
                NS(
                    name=prefix + str(side),
                    data=NS(vertices=vertices, update=lambda: None),
                )
            )
        exec(
            textwrap.dedent(block),
            dict(
                PARTS=parts,
                H=1.0,
                C=dict(eye_center_x=0.04, eye_half_width=0.02),
                STYLE=(
                    {} if lift_scale is None else {"accent_wing_lift_scale": lift_scale}
                ),
            ),
        )
        for vertex, prefix, t, side, offset in records:
            u = min(1, max(0, (t - 0.65) / 0.2))
            scale = (
                1 - 0.35 * u * u * (3 - 2 * u) if prefix.startswith("Attached") else 1
            )
            tip = min(1, max(0, (t - 1.0) / 0.1))
            lift = (
                0.012
                * (1 if lift_scale is None else lift_scale)
                * tip
                * tip
                * (3 - 2 * tip)
            )
            self.assertAlmostEqual(vertex.co.x, side * (0.04 + 0.02 * t))
            self.assertEqual(vertex.co.y, -0.03)
            self.assertAlmostEqual(
                vertex.co.z,
                (
                    (1.598 + (offset + 0.002) * scale + lift)
                    if prefix.startswith("Attached")
                    else 1.6 + offset
                ),
            )
