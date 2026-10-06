"""Pure mathematical checks; no Blender import or asset generation."""
import ast
import copy
import json
import math
import unittest
from pathlib import Path

ROOT = Path(__file__).parent
STYLE = json.loads((ROOT / 'face_style.json').read_text())


def pure_function(module, name, extra):
    tree = ast.parse((ROOT / module).read_text())
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == name]
    scope = {key: getattr(math, key) for key in ['sin', 'cos', 'pi', 'exp', 'sqrt', 'atan2']}
    scope.update(extra)
    exec(compile(ast.Module(body=nodes, type_ignores=[]), '<pure style math>', 'exec'), scope)
    return scope[name], scope


class StyleMathTests(unittest.TestCase):
    def test_ear_outline_is_periodic_and_affine(self):
        fn, scope = pure_function('ears.py', 'outline', {'C': copy.deepcopy(STYLE['calibration'])})
        shift = [.02, -.03, .01]
        samples = [i * math.pi / 9 for i in range(18)]
        original = [fn(t) for t in samples]
        for t, point in zip(samples, original):
            for a, b in zip(point, fn(t + 2 * math.pi)):
                self.assertAlmostEqual(a, b, places=10)
        points = scope['C']['ear_outline_stations']
        scope['C']['ear_outline_stations'] = [[p[i] + shift[i] for i in range(3)] for p in points]
        for t, point in zip(samples, original):
            for i, actual in enumerate(fn(t)):
                self.assertAlmostEqual(actual, point[i] + shift[i], places=10)

    def test_iris_palette_is_finite_and_bounded(self):
        fn, _ = pure_function('textures.py', 'iris_color', {'STYLE': STYLE})
        for iy in range(17):
            for ix in range(17):
                rgba = fn(ix / 16, iy / 16)
                self.assertEqual(rgba[3], 1)
                self.assertTrue(all(math.isfinite(c) and 0 <= c <= 1 for c in rgba))

    def test_ear_palette_is_bounded_without_reference_pixels(self):
        fn, _ = pure_function('textures.py', 'ear_color', {'STYLE': STYLE, 'SKIN': STYLE['skin_rgb']})
        for iy in range(21):
            for ix in range(21):
                rgba = fn(ix/20, iy/20)
                self.assertEqual(rgba[3], 1)
                self.assertTrue(all(math.isfinite(c) and 0 <= c <= 1 for c in rgba))

    def test_simple_ear_band_contract(self):
        calibration = STYLE['calibration']
        bands = calibration['ear_relief_bands']
        self.assertEqual(calibration['ear_ring_samples'], 24)
        self.assertEqual(len(bands), 3)
        self.assertTrue(all(a[0] < b[0] for a,b in zip(bands,bands[1:])))
        self.assertEqual(bands[-1][0], 1)
        self.assertTrue(all(abs(b[1]) <= .004 for b in bands))


if __name__ == '__main__':
    unittest.main()
