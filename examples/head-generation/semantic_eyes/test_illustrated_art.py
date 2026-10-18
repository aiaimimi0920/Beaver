"""CPU checks for the opt-in eye paint; native geometry still requires review."""

import math
import ast
import json
from parameters import adapt_style
import unittest
from illustrated_art import IRIS, adapt_sources
from pathlib import Path


class IllustratedArtTests(unittest.TestCase):
    def setUp(self):
        self.ns = dict(vars(math))
        self.ns.update(
            WHITE=(0.95, 0.94, 0.92),
            Y0=0.0,
            H=1.0,
            C={"eye_corner_y": 0.0},
            eye_contour=lambda side, t, upper: (t, 0.1),
        )
        exec(IRIS, self.ns)

    def test_paint_is_finite_and_bounded(self):
        for fn in ("iris_color", "sclera_art_color"):
            for i in range(41):
                for j in range(41):
                    c = self.ns[fn](i / 40, j / 40)
                    self.assertEqual(c[3], 1)
                    self.assertTrue(all(math.isfinite(x) and 0 <= x <= 1 for x in c))

    def test_lid_darkness_and_lower_iris_light(self):
        iris = self.ns["iris_color"]
        sclera = self.ns["sclera_art_color"]
        self.assertGreater(sum(iris(0.5, 0.2)[:3]), sum(iris(0.5, 0.8)[:3]))
        self.assertLess(sum(sclera(0.5, 0.7)[:3]), sum(sclera(0.5, 0.3)[:3]))

    def test_source_rewrite_is_explicit_and_does_not_mutate(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        source = {
            name: (root / name).read_text()
            for name in ("textures.py", "pipeline.py", "eye_socket.py")
        }
        before = dict(source)
        result = adapt_sources(source)
        self.assertEqual(source, before)
        for name in source:
            self.assertNotEqual(result[name], source[name])
            compile(result[name], name, "exec")

    def test_control_map_classifies_sclera(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        source = {
            name: (root / name).read_text()
            for name in ("textures.py", "pipeline.py", "eye_socket.py")
        }
        result = adapt_sources(source)["textures.py"]
        self.assertIn("or sclera_island", result)
        self.assertIn("0.405 < (py + 0.5) / 256 < 0.495", result)

    def test_outline_holdout_uses_reserved_atlas_islands(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        sources = adapt_sources(
            {
                name: (root / name).read_text()
                for name in ("textures.py", "pipeline.py", "eye_socket.py")
            }
        )
        self.assertIn("face_outline_control = ExtResource", sources["pipeline.py"])
        tree = ast.parse(sources["textures.py"])
        wanted = {
            "flat_region",
            "iris_island",
            "sclera_island",
            "ink_island",
            "holdout",
        }
        assignments = [
            n
            for n in ast.walk(tree)
            if isinstance(n, ast.Assign)
            and len(n.targets) == 1
            and isinstance(n.targets[0], ast.Name)
            and n.targets[0].id in wanted
        ]
        for u, v, expected in [
            (0.3, 0.95, False),
            (0.8, 0.7, False),
            (0.8, 0.94, True),
            (0.8, 0.2, True),
            (0.8, 0.45, True),
            (0.8, 0.875, True),
        ]:
            ns = {"u": u, "py": v * 256 - 0.5}
            for node in assignments:
                exec(
                    compile(
                        ast.Module(body=[node], type_ignores=[]), "mask-test", "exec"
                    ),
                    ns,
                )
            self.assertEqual(ns["holdout"], expected)

    def test_neutral_profile_still_applies_and_keeps_anatomy(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        base = json.loads((root / "face_style.json").read_text())
        self.assertEqual(adapt_style(base, {}), base)
        marked = dict(base, illustrated_eye_art=True)
        result = adapt_style(marked, {})
        self.assertNotEqual(result["upper_lash_rails"], base["upper_lash_rails"])
        self.assertEqual(result["calibration"], base["calibration"])
        near = adapt_style(marked, {"brow_height_offset": 1e-9})
        self.assertEqual(near["upper_lash_rails"], result["upper_lash_rails"])

    def test_level_corners_and_restrained_tail(self):
        root = Path(__file__).parents[1] / "checkpoints/current"
        base = json.loads((root / "face_style.json").read_text())
        request = dict(
            corner_slope_offset=-0.014,
            upper_arch=1.4,
            lash_band_thickness=0.92,
            lash_contour_follow=1,
            supplemental_lashes=0,
            short_upper_tuft=1,
        )
        result = adapt_style(dict(base, illustrated_eye_art=True), request)
        self.assertAlmostEqual(result["calibration"]["eye_corner_slope"], 0.0)
        self.assertAlmostEqual(result["upper_lash_rails"][-1][0], 1.025)
        self.assertAlmostEqual(result["upper_lash_rails"][-1][1], 0.012)
        c = result["calibration"]
        for i, (t, lo, hi) in enumerate(result["upper_lash_rails"]):
            if not 0.75 < t <= 0.96:
                continue
            q = i / 20
            arch = (
                max(0.0, 1 - abs(t) ** c["eye_horizontal_power"])
                ** c["eye_upper_power"]
            )
            width = (
                0.018
                * request["lash_band_thickness"]
                * math.sin(math.pi * q) ** 0.8
                * (0.45 + 0.90 * q)
            )
            expected = (
                c["eye_upper_arch"] * arch
                - 0.002 * math.sin(math.pi * q)
                - 0.55 * width
            )
            self.assertAlmostEqual(lo, expected)
        outer = [row for row in result["upper_lash_rails"] if row[0] > 0.75]
        for lane in (1, 2):
            self.assertTrue(all(a[lane] >= b[lane] for a, b in zip(outer, outer[1:])))
        for boundary in (-0.018, 0.008):
            adapt_style(base, dict(request, corner_slope_offset=boundary))
        for bad in (-0.019, 0.009):
            with self.assertRaises(ValueError):
                adapt_style(base, dict(request, corner_slope_offset=bad))


if __name__ == "__main__":
    unittest.main()
