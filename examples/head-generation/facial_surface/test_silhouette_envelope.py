"""Execute the retained ear envelope target expressions in isolation."""
import ast
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from source_silhouette import adapt_ear_envelope, apply_silhouette
ROOT = Path(__file__).parent.parent / 'checkpoints/current'

class EnvelopeTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((ROOT / 'face_style.json').read_text())
        self.original = self.style['calibration']['width_stations']
        curve = next(n for n in ast.parse((ROOT / 'face.py').read_text()).body
                     if isinstance(n, ast.FunctionDef) and n.name == 'curve')
        ns = {}
        exec(compile(ast.Module(body=[curve], type_ignores=[]), 'curve', 'exec'), ns)
        self.curve = ns['curve']
        source = adapt_ear_envelope((ROOT / 'ear_width.py').read_text(), self.original)
        nodes = [n for n in ast.walk(ast.parse(source)) if isinstance(n, ast.Assign)
                 and isinstance(n.targets[0], ast.Name)
                 and n.targets[0].id in ('v', 'silhouette_delta', 'delta')]
        self.code = compile(ast.Module(body=nodes, type_ignores=[]), 'targets', 'exec')

    def calculate(self, controls, h=.16, H=.236, Y0=1.544, current=.06):
        style, _ = apply_silhouette(self.style, controls)
        ns = dict(p=SimpleNamespace(z=Y0+H*h-.021), H=H, Y0=Y0,
                  C=style['calibration'], curve=self.curve, w=.25, height=.236, current=current)
        exec(self.code, ns)
        return ns

    def test_zero_matches_original_and_saturation(self):
        for h in (.03,.16,.20,.45):
            for current in (.01,.06,.10):
                ns=self.calculate({},h=h,current=current)
                self.assertEqual(ns['silhouette_delta'],0)
                self.assertEqual(ns['delta'],max(-.003,min(.003,.25*.236-current)))

    def test_units_sign_and_translation(self):
        for h,expected in ((.16,.00130272),(.20,.0013688)):
            for sign in (-1,1):
                for origin in (1.544,2.544):
                    ns=self.calculate({'jaw_width':sign*.02},h=h,Y0=origin)
                    self.assertAlmostEqual(ns['silhouette_delta'],sign*expected,places=12)
                ns=self.calculate({'jaw_width':sign*.02},h=h,H=.472)
                self.assertAlmostEqual(ns['silhouette_delta'],sign*2*expected,places=12)

if __name__ == '__main__':
    unittest.main()
