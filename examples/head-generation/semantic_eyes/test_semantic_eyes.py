"""CPU recipe/geometry-function tests. No Blender or Godot execution."""
import ast
import copy
import json
import math
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from parameters import adapt_style, validate, LIMITS, upper_change
from bundle import build_request, RECIPE, REQUIRED

TARGET = {'eye_spacing': 1., 'eye_width': 1.04, 'upper_arch': .78, 'lower_arch': .70,
          'corner_slope_offset': .002, 'eye_height_offset': 0., 'iris_width': .96,
          'iris_height': .92, 'brow_thickness': .70, 'brow_arch': 1.10,
          'brow_height_offset': .004, 'brow_rise_offset': .004,
          'lash_band_thickness': .65, 'supplemental_lashes': 0, 'lid_fold': 0}


class SemanticEyeTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / 'face_style.json').read_text())

    def contour(self, style):
        tree = ast.parse((RECIPE / 'face.py').read_text())
        node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'eye_contour')
        c = style['calibration']
        scope = {'C': c, 'H': c['height_m'], 'Y0': c['origin_y_m']}
        exec(compile(ast.Module(body=[node], type_ignores=[]), '<actual eye contour>', 'exec'), scope)
        return scope['eye_contour']

    def test_identity_copy_and_invalid(self):
        original = copy.deepcopy(self.style)
        self.assertEqual(adapt_style(self.style, {}), original)
        altered = adapt_style(self.style, TARGET)
        self.assertEqual(self.style, original)
        self.assertNotEqual(altered, original)
        for key, value in [('unknown', 1), ('eye_width', True), ('upper_arch', math.nan),
                           ('eye_width', 1.2), ('supplemental_lashes', .5)]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                adapt_style(self.style, {key: value})

    def test_target_actual_contour_ratios_and_coupling(self):
        changed = adapt_style(self.style, TARGET)
        base, target = self.contour(self.style), self.contour(changed)
        for side in (-1, 1):
            self.assertAlmostEqual(target(side, 0, True)[0], base(side, 0, True)[0])
            self.assertAlmostEqual((target(side, 1, True)[0] - target(side, -1, True)[0]) /
                                   (base(side, 1, True)[0] - base(side, -1, True)[0]), 1.04)
        old, new = self.style['calibration'], changed['calibration']
        for key, scale in [('eye_upper_arch', .78), ('eye_lower_arch', .70)]:
            self.assertAlmostEqual(new[key] / old[key], scale)
        self.assertAlmostEqual(changed['eye_half_width_m'], new['eye_half_width'] * new['height_m'])
        self.assertAlmostEqual(changed['iris_half_width_m'], new['iris_half_width'] * new['height_m'])
        self.assertEqual(old['mouth_height'], new['mouth_height'])
        self.assertEqual(old['nose_stations'], new['nose_stations'])
        for old_row, new_row in zip(self.style['lid_fold_height_profile'], changed['lid_fold_height_profile']):
            t = -.76 + 1.52 * old_row[0]
            expected = old_row[1] + upper_change(t, old, validate(TARGET)) + .002 * t
            self.assertAlmostEqual(new_row[1], expected)
        for key in LIMITS:
            for bound in LIMITS[key][1:]:
                # Single-axis supported endpoints must be valid; combinations
                # are additionally protected by the geometric envelope guard.
                adapt_style(self.style, {key: bound})

    def test_optional_geometry_is_removed_before_assembly(self):
        tree = ast.parse(Path(__file__).with_name('runtime.py').read_text())
        tree.body = [n for n in tree.body if not isinstance(n, (ast.Import, ast.ImportFrom))]
        removed, repaired = [], []
        scope = {'bpy': SimpleNamespace(data=SimpleNamespace(objects=SimpleNamespace(
                    remove=lambda obj, do_unlink: removed.append(obj.name)))),
                 'store_mouth_deltas': lambda obj: repaired.append(obj.name)}
        exec(compile(tree, '<native selection mock>', 'exec'), scope)
        names = ['Upper lash 1', 'Upper skin lid 1', 'Upper swept lash tuft 1',
                 'Embedded lateral child 0 1', 'Tapered upper eyelid fold 1']
        parts = [SimpleNamespace(name=name, data=SimpleNamespace(attributes={})) for name in names]
        result = scope['select_components'](parts, validate(TARGET))
        self.assertEqual([p.name for p in parts], names[:2])
        self.assertEqual(removed, names[2:])
        self.assertEqual(repaired, names[:2])
        self.assertEqual(result['removed_components'], names[2:])

    def test_fresh_package_snapshot_hashes_and_no_final_restore(self):
        before = {p.name: p.read_bytes() for p in RECIPE.glob('*.py')}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('CPU packaging fixture; never execute')
            prefix = 'assets/aster/general_face_eyes_test/'
            previous = root / prefix / 'aster_head.glb'
            previous.parent.mkdir(parents=True)
            previous.write_bytes(b'previous output')
            request = build_request(TARGET, prefix, root)
            compile(request['code'], '<native request>', 'exec')
            self.assertNotIn('head_recovery_617', request['code'])
            calls = [n.func.id for n in ast.walk(ast.parse(request['code']))
                     if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)]
            self.assertNotIn('preserve_non_eye', calls)
            self.assertNotIn('refine_outer_eye', calls)
            self.assertIn('select_components(PARTS', request['code'])
            code = request['code']
            self.assertLess(code.index('report["ear_root_fit"] = fit_ear_roots(shell)'),
                            code.index('semantic_part_ordinals ='))
            self.assertLess(code.index('semantic_part_ordinals ='), code.index('select_components(PARTS'))
            self.assertIn('region = semantic_part_ordinals[part.name]', code)
            paths = {o['path']: o['expectedSha256'] for o in request['outputs']}
            self.assertIsNotNone(paths[prefix + 'aster_head.glb'])
            self.assertIn(prefix + 'input-manifest.json', paths)
            self.assertIn(prefix + 'semantic_face_export.glb', paths)
            self.assertIn('/ (0.044 * 0.92) + 0.5', code)
            self.assertIn('iv = y / (0.036 * 0.92) + 0.5', code)
            self.assertLess(code.index('Recipe input changed:'), code.index('import copy, types, sys'))
            verification = code.split('import copy, types, sys', 1)[0]
            exec(verification, {'beaver_input': lambda name: str(root / name)})
            (root / REQUIRED[0]).write_text('changed')
            with self.assertRaisesRegex(AssertionError, 'Recipe input changed'):
                exec(verification, {'beaver_input': lambda name: str(root / name)})
            (root / REQUIRED[0]).unlink()
            with self.assertRaisesRegex(ValueError, 'Missing'):
                build_request(TARGET, prefix, root)
        self.assertEqual(before, {p.name: p.read_bytes() for p in RECIPE.glob('*.py')})


if __name__ == '__main__':
    unittest.main()
