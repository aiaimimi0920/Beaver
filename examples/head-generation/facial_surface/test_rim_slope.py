"""Rim rotation remains in the shared field and the reviewed displacement budget."""
import ast
import copy
import json
from pathlib import Path
import tempfile
import unittest
from hypothesis import validate
from runtime_field import rim_slope_delta, base_delta
from preflight import prepare
from study import adapt_style, scope, fit_profile
from bundle import build_request
from test_upper_warp_bundle import retained_sources

class RimSlopeTests(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        root = Path(__file__).parents[1]
        cls.source = (root / 'checkpoints/current/face.py').read_text()
        cls.style = json.loads((root / 'checkpoints/current/face_style.json').read_text())
        cls.eyes = json.loads((root / 'facial_surface/emerald-asymmetric-eyes.json').read_text())
        cls.c = adapt_style(cls.style, cls.eyes)['calibration']
        from source_upper_warp import adapt_face_source
        cls.source = adapt_face_source(cls.source, True)
        cls.p = dict(axial_smoothing=0.5, cheek_roundness=0.05, orbit_shallowing=0.15)
        cls.data0, cls.report0 = prepare(cls.source, cls.c, cls.p)
        cls.data1, cls.report1 = prepare(cls.source, cls.c, dict(cls.p, orbit_rim_flattening=0.15))

    def test_validation_identity_and_unclamped_pivot(self):
        for v in (-0.01, 0.15001, True, float('nan')):
            with self.assertRaises(ValueError):
                validate({'orbit_rim_flattening': v})
        c = self.c
        k = -0.15 * 0.85 * c['height_m'] * (c['eye_outer_depth'] - c['eye_inner_depth']) / 2
        for t in (-2, -1, 0, 1, 2):
            x = c['height_m'] * (c['eye_center_x'] + c['eye_half_width'] * t)
            self.assertEqual(rim_slope_delta(x, c, self.data0), 0)
            self.assertAlmostEqual(rim_slope_delta(x, c, self.data1), k * t, places=14)
            self.assertEqual(rim_slope_delta(-x, c, self.data1), rim_slope_delta(x, c, self.data1))

    def test_preflight_retains_original_and_fits_full_ocular_delta(self):
        explicit0, report0 = prepare(self.source, self.c, dict(self.p, orbit_rim_flattening=0))
        self.assertEqual(explicit0, self.data0)
        self.assertEqual(report0, self.report0)
        before = copy.deepcopy(self.c)
        data, report = prepare(self.source, self.c, dict(self.p, orbit_rim_flattening=0.15))
        self.assertEqual(self.c, before)
        c = self.c
        k = -0.15 * 0.85 * c['height_m'] * (c['eye_outer_depth'] - c['eye_inner_depth']) / 2
        a0, b0, d0 = self.data0['ocular_plane']['coefficients']
        a1, b1, d1 = data['ocular_plane']['coefficients']
        self.assertAlmostEqual(a1 - a0, k / (c['height_m'] * c['eye_half_width']), places=12)
        self.assertAlmostEqual(b1 - b0, 0, places=12)
        self.assertAlmostEqual(d1 - d0, k * (c['iris_center_x'] - c['eye_center_x']) / c['eye_half_width'], places=12)
        s0, s1 = (self.report0['surface'], report['surface'])
        self.assertGreaterEqual(s1['additional_rim_slope_skin_bound_m'], abs(k) * (1 + c['orbital_lateral_fade']))
        self.assertAlmostEqual(s1['aperture_rim_displacement_bound_m'] - s0['aperture_rim_displacement_bound_m'], abs(k), places=10)
        self.assertGreater(s1['conservative_support_displacement_bound_m'], s0['conservative_support_displacement_bound_m'])

    def test_packaged_native_rim_agrees_and_budget_is_enforced(self):
        p = dict(self.p, orbit_rim_flattening=0.15)
        old_bound = self.report0['surface']['conservative_support_displacement_bound_m']
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with self.assertRaisesRegex(ValueError, 'exceeds explicitly reviewed'):
                build_request(p, self.eyes, 'assets/aster/general_face_eyes_rim_cpu/', root, old_bound)
            names = ['project.godot', 'authoring/head_generation_guide.md'] + ['assets/aster/head_recovery_49/' + n for n in ['aster_head_editable.blend', 'aster_head.glb', 'aster_definition.tres', 'hair_base.png', 'hair_ilm.png']]
            for name in names:
                f = root / name
                f.parent.mkdir(parents=True, exist_ok=True)
                f.write_text('CPU fixture')
            request = build_request(p, self.eyes, 'assets/aster/general_face_eyes_rim_cpu/', root, 0.0042, 'emerald_illustrated', blink=True)
        retained = retained_sources(request['code'])
        self.assertIn('rim_slope_delta(x, C, DATA)', retained['face.py'])
        self.assertIn('def rim_slope_delta(', request['code'])
        old = scope(self.source, self.c, fit_profile(self.source, self.c), 0)
        env = dict(old)
        env.update(C=self.c, DATA=self.data1, REVIEWED_BOUND=0.0042, inherited_rim_depth=old['rim_depth'], inherited_base_depth=old['base_depth'], base_delta=base_delta, rim_slope_delta=rim_slope_delta, curve=old['curve'])
        node = next((n for n in ast.parse(retained['face.py']).body if isinstance(n, ast.FunctionDef) and n.name == 'rim_depth'))
        exec(compile(ast.Module(body=[node], type_ignores=[]), '<actual native rim>', 'exec'), env)
        for t in (-1, 0, 1):
            c = self.c
            x = c['height_m'] * (c['eye_center_x'] + c['eye_half_width'] * t)
            y = c['origin_y_m'] + c['height_m'] * c['eye_corner_y']
            expected = old['rim_depth'](x, y) + base_delta(x, y, c, old['curve'], self.data1) + 0.15 * (old['base_depth'](x, y) - old['rim_depth'](x, y)) + rim_slope_delta(x, c, self.data1)
            self.assertAlmostEqual(env['rim_depth'](x, y), expected, places=14)
        with self.assertRaisesRegex(ValueError, 'reviewed displacement bound'):
            env['rim_depth'](self.c['height_m'] * (self.c['eye_center_x'] + 8 * self.c['eye_half_width']), y)
        depth_node = next((n for n in ast.parse(retained['face.py']).body if isinstance(n, ast.FunctionDef) and n.name == 'depth'))
        env['base_depth'] = lambda x, y: old['base_depth'](x, y) + base_delta(x, y, self.c, old['curve'], env['DATA'])
        exec(compile(ast.Module(body=[depth_node], type_ignores=[]), '<actual native depth>', 'exec'), env)
        for t in (-1.2, -1, -0.6, 0, 0.6, 1, 1.159, 1.161):
            xx = self.c['height_m'] * (self.c['eye_center_x'] + self.c['eye_half_width'] * t)
            for dy in (-0.01, 0, 0.01):
                value = env['depth'](xx, y + dy)
                self.assertLessEqual(abs(value - old['depth'](xx, y + dy)), self.report1['surface']['maximum_depth_displacement_m'])
        env['DATA'] = self.data0
        zero_implicit = [env['depth'](x, y), env['rim_depth'](x, y)]
        explicit0, _ = prepare(self.source, self.c, dict(self.p, orbit_rim_flattening=0))
        env['DATA'] = explicit0
        self.assertEqual(zero_implicit, [env['depth'](x, y), env['rim_depth'](x, y)])
        env['DATA'] = self.data1
        env['REVIEWED_BOUND'] = 1e-10
        with self.assertRaisesRegex(ValueError, 'reviewed displacement bound'):
            env['depth'](x, y)
        with self.assertRaisesRegex(ValueError, 'reviewed displacement bound'):
            env['rim_depth'](x, y)
        self.assertEqual(len(request['outputs']), 64)
if __name__ == '__main__':
    unittest.main()
