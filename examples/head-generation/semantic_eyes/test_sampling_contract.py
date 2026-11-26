"""First-pass sampling contracts and real ribbon construction, without Blender."""
import ast
import importlib.util
import json
import math
from pathlib import Path
import sys
import unittest

from parameters import adapt_style, validate
from sampling_contract import validate_sampling, lid_station_plan

HERE = Path(__file__).resolve().parent
RECIPE = HERE.parent / 'checkpoints/current'


def functions(path, names, scope=None):
    namespace = {} if scope is None else scope
    tree = ast.parse(path.read_text())
    body = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert len(body) == len(names)
    exec(compile(ast.Module(body=body, type_ignores=[]), str(path), 'exec'), namespace)
    return namespace


class SamplingTests(unittest.TestCase):
    def test_reject_invalid_controls(self):
        for request in ({'unknown': 2}, {'sampling_rim_quality_guard': 1}, {'sampling_main_stations': 10},
                        {'sampling_companion_stations': True},
                        {'sampling_companion_stations': 8.5},
                        {'sampling_wing_gap': float('nan')},
                        {'sampling_lateral_gap': .4}, {'sampling_first_pass': .5}, {'sampling_iris_samples': 31}, {'sampling_iris_rings': 1}, {'sampling_shell_columns': 11}):
            with self.subTest(request=request), self.assertRaises(ValueError):
                validate_sampling(request)
        with self.assertRaises(ValueError):
            validate({'sampling_main_stations': 10})

    def test_mode_survives_validated_parameter_roundtrip(self):
        for mode in (0, 1):
            request = {'sampling_first_pass': mode, 'sampling_companion_stations': 9}
            first = validate(request)
            self.assertEqual(first, validate(json.loads(json.dumps(first))))
            self.assertEqual(first['sampling_first_pass'], mode)
        self.assertEqual(validate({})['sampling_first_pass'], 0)

    def test_all_controls_survive_style_roundtrip(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        controls = {'sampling_first_pass': 1, 'sampling_shell_columns': 9, 'sampling_main_stations': 13,
                    'sampling_companion_stations': 9, 'sampling_lateral_gap': .13,
                    'sampling_wing_gap': .10, 'sampling_iris_samples': 32,
                    'sampling_iris_rings': 2, 'sampling_pocket_samples': 32,
                    'sampling_lid_stations': 17, 'sampling_brow_divisions': 16}
        emitted = adapt_style(original, controls)
        replay = adapt_style(original, json.loads(json.dumps(validate(controls))))
        self.assertEqual(emitted, replay)
        for key, value in controls.items():
            self.assertEqual(emitted[key], value)

    def test_legacy_mode_does_not_disable_lash_gap_refinement(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        legacy = adapt_style(original, {'lash_contour_follow': 1, 'sampling_first_pass': 0})
        reduced = adapt_style(original, {'lash_contour_follow': 1, 'sampling_first_pass': 1})
        self.assertFalse(legacy.get('sampling_main_exact', False))
        self.assertTrue(reduced['sampling_main_exact'])
        self.assertEqual(legacy['upper_lash_rails'], reduced['upper_lash_rails'])

    def test_shell_retained_stations_keep_nominal_positions(self):
        tree = ast.parse((RECIPE / 'shell.py').read_text())
        build = next(node for node in tree.body
                     if isinstance(node, ast.FunctionDef) and node.name == 'build_face')
        setup = []
        for node in build.body:
            setup.append(node)
            if isinstance(node, ast.Assign) and any(
                    isinstance(target, ast.Tuple) and
                    [getattr(item, 'id', None) for item in target.elts] == ['rows', 'cols']
                    for target in node.targets):
                break
        variants = []
        for count in (13, 9):
            scope = {'STYLE': {'sampling_shell_columns': count}}
            exec(compile(ast.Module(body=setup, type_ignores=[]), 'shell_sampling', 'exec'), scope)
            self.assertEqual(scope['cols'], count)
            self.assertEqual(scope['rows'], 22)
            self.assertTrue({0, .18, .44, .94, 1} <= set(scope['lateral']))
            variants.append(scope)
        old, new = variants
        for i, original_index in enumerate(new['retained']):
            for focus in (0, .25, .5, 1):
                original = old['lateral'][original_index]*focus + old['nominal'][original_index]*(1-focus)
                reduced = new['lateral'][i]*focus + new['nominal'][i]*(1-focus)
                self.assertEqual(original, reduced)

    def test_emitted_reduced_lid_stations(self):
        from source_angular_samples import adapt_angular_samples
        sources = {name: (RECIPE / name).read_text()
                   for name in ('face.py', 'eye_socket.py')}
        for angular in (False, True):
            result = adapt_angular_samples(sources, angular,
                                          {'sampling_lid_stations': 17})
            tree = ast.parse(result['face.py'])
            values = [ast.literal_eval(node.value) for node in ast.walk(tree)
                      if isinstance(node, ast.Assign)
                      and any(isinstance(target, ast.Name) and target.id == 'stations'
                              for target in node.targets)
                      and isinstance(node.value, ast.List)]
            self.assertIn(list(lid_station_plan(17, angular)), values)
            self.assertNotIn('        cols = 33', result['face.py'])

    def test_shared_companion_layout_and_actual_triangle_counts(self):
        runtime = functions(RECIPE / 'lateral_fit.py', ['companion_stations'])
        ribbon = functions(RECIPE / 'thin_ribbon.py',
                           ['closed_ribbon_mesh', 'signed_volume'])
        for count in (5, 9, 13, 17):
            stations = runtime['companion_stations']({'sampling_companion_stations': count})
            self.assertEqual(len(stations), count)
            self.assertEqual((stations[0], stations[-1]), (0, 1))
            self.assertIn(.5, stations)
            sections = [(f, 0, math.sin(math.pi * f)) for f in stations]
            vertices, faces, zones = ribbon['closed_ribbon_mesh'](
                sections, lambda f, height, relief: (f, height, relief), .01, .002)
            self.assertEqual(len(faces), 12 * (count - 2))
            self.assertEqual(len(vertices), 6 * (count - 2) + 2)
            self.assertEqual(len(zones), len(faces))

    def test_main_sampling_keeps_endpoints_and_middle(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        controls = {'lash_contour_follow': 1, 'layered_lashes': 1,
                    'supplemental_lashes': 0, 'lid_fold': 0, 'short_upper_tuft': 1}
        default = adapt_style(original, controls)
        old = default['upper_lash_rails']
        for count in (9, 13, 17):
            adapted = adapt_style(original, dict(controls, sampling_main_stations=count))
            rails = adapted['upper_lash_rails']
            self.assertEqual(len(rails), count)
            native = functions(RECIPE / 'thin_ribbon.py', ['accent_sections'], {'STYLE': adapted})
            self.assertEqual(native['accent_sections']('upper_lash_rails'), [tuple(row) for row in rails])
            self.assertEqual(rails[0], old[0])
            self.assertEqual(rails[-1], old[-1])
            self.assertEqual(rails[count // 2], old[10])
            self.assertTrue(all(a[0] < b[0] for a, b in zip(rails, rails[1:])))

    def test_actual_lateral_sections_default_match_frozen_contract(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        legacy = {}
        exec((RECIPE / 'sampling_legacy_contract.py').read_text(), legacy)
        restored = legacy['legacy_accent_tree'](ast.parse((RECIPE / 'eye_accents.py').read_text()))
        old = next(n for n in restored.body if isinstance(n, ast.FunctionDef)
                   and n.name == 'lateral_sections')
        curve = functions(RECIPE / 'face.py', ['curve'])['curve']
        def sections(style, frozen=False):
            scope = {'STYLE':style, 'C':style['calibration'], 'curve':curve}
            if frozen:
                exec(compile(ast.Module(body=[old], type_ignores=[]), 'frozen_lateral', 'exec'), scope)
            else:
                functions(RECIPE / 'eye_accents.py', ['lateral_sections'], scope)
            return scope['lateral_sections']()
        expected = sections(original, frozen=True)
        self.assertEqual(sections(original), expected)
        self.assertEqual(sections(dict(original, sampling_lateral_gap=.065)), expected)

    def test_actual_companion_builder_and_fitter_default_match_frozen_contract(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        legacy = {}
        exec((RECIPE / 'sampling_legacy_contract.py').read_text(), legacy)
        tree = ast.parse((RECIPE / 'eye_accents.py').read_text())
        restored = legacy['legacy_accent_tree'](tree)
        old_builder = next(n for n in restored.body if isinstance(n, ast.FunctionDef)
                           and n.name == 'build_lateral_accent')
        current_counts = next(n.value for n in ast.walk(tree)
                              if isinstance(n, ast.Assign)
                              and any(isinstance(t, ast.Name) and t.id == 'counts' for t in n.targets)
                              and 'companion_stations' in ast.dump(n.value))
        runtime = functions(RECIPE / 'lateral_fit.py', ['companion_stations'])
        ribbon = functions(RECIPE / 'thin_ribbon.py', ['closed_ribbon_mesh', 'signed_volume'])
        def build(style, old=False):
            captured = {}
            def capture(name, sections, point, color, thickness, roll, closed):
                if name.startswith('Outer companion lash '):
                    captured['sections'] = sections
                    captured['mesh'] = ribbon['closed_ribbon_mesh'](sections, point, thickness, roll)
            scope = {'STYLE':style, 'sin':math.sin, 'pi':math.pi, 'accent_mesh':capture,
                     'lateral_sections':lambda: [],
                     'lateral_accent_point':lambda side,f,w,r: (side*f,w,r), **runtime}
            if old:
                exec(compile(ast.Module(body=[old_builder], type_ignores=[]), 'frozen_builder', 'exec'), scope)
            else:
                functions(RECIPE / 'eye_accents.py', ['build_lateral_accent'], scope)
            scope['build_lateral_accent'](1)
            return captured
        baseline = build(original, old=True)
        self.assertEqual(build(original), baseline)
        self.assertEqual(build(dict(original, sampling_companion_stations=17)), baseline)
        for count in (9, 17):
            style = dict(original, sampling_companion_stations=count)
            result = build(style)
            counts = eval(compile(ast.Expression(current_counts), 'actual_fitter_counts', 'eval'),
                          {'STYLE':style, **runtime})
            self.assertEqual(sum(counts), len(result['mesh'][0]))
            self.assertEqual(sum(counts), 6*(count-2)+2)
            self.assertEqual(len(result['mesh'][1]), 12*(count-2))

    def test_lid_plan_preserves_required_anchors(self):
        for count in (17, 25, 33):
            plain = lid_station_plan(count)
            angular = lid_station_plan(count, True)
            self.assertEqual(len(plain), count)
            self.assertTrue({-1, 0, 1} <= set(plain))
            self.assertTrue({-.55, .55} <= set(angular))
            self.assertTrue(set(plain) <= set(angular))
        with self.assertRaises(ValueError):
            lid_station_plan(18)

    def test_native_iris_sampling_retains_bevel_radii_and_counts(self):
        original = json.loads((RECIPE / 'face_style.json').read_text())
        adapted = adapt_style(original, {'sampling_iris_samples': 32, 'sampling_iris_rings': 2,
                                          'sampling_pocket_samples': 32})
        scope = {'C': adapted['calibration'], 'H': 1.0, 'Y0': 0.0,
                 'coord': lambda x,y,z: (x,y,z), 'iris_depth': lambda x,y: 0.0,
                 'pi': math.pi, 'sin': math.sin, 'cos': math.cos}
        native = functions(RECIPE / 'eye_socket.py', ['annular_iris_mesh'], scope)
        vertices, polygons = native['annular_iris_mesh'](1)
        self.assertEqual(len(vertices), 65)
        self.assertEqual(sum(len(poly)-2 for poly in polygons), 96)
        c = adapted['calibration']
        self.assertAlmostEqual(vertices[1][0], c['iris_center_x'] + c['iris_half_width']*.94)
        self.assertAlmostEqual(vertices[33][0], c['iris_center_x'] + c['iris_half_width'])
        self.assertEqual(c['pocket_ring_samples'], 32)
        inherited = adapt_style(original, {})
        for key in ('iris_radial_samples', 'iris_radial_rings', 'pocket_ring_samples'):
            self.assertEqual(inherited['calibration'][key], original['calibration'][key])

    def test_snapshot_changes_and_default_is_unchanged(self):
        sys.path.insert(0, str(HERE.parent / 'face_components'))
        try:
            spec = importlib.util.spec_from_file_location('sampling_test_compose',
                HERE.parent / 'face_components/compose.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            selection = {'eyes': 'almond', 'brows': 'research-angled',
                         'nose': 'research-compact', 'mouth': 'research-lip-band',
                         'ears': 'research-slender-oval', 'face': 'research-straight-tapered'}
            default = module.resolve(selection)
            self.assertEqual(default, module.resolve(selection, sampling_overrides=None))
            changed = module.resolve(selection, sampling_overrides={'sampling_companion_stations': 9})
            self.assertNotEqual(default['composition_sha256'], changed['composition_sha256'])
            self.assertEqual(changed['eye_parameters']['sampling_companion_stations'], 9)
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
