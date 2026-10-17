"""CPU checks for the opt-in eye paint; native geometry still requires review."""

import math
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


if __name__ == "__main__":
    unittest.main()
