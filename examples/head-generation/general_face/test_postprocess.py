"""CPU-only alias regression test; this does not run or impersonate Blender."""
import ast
from pathlib import Path
from types import SimpleNamespace
import unittest
from controls import (transform_point, mouth_delta, inverse_transpose_normal,
                      measure_profile, PRESETS)


class Blocks(list):
    def keys(self):
        return [key.name for key in self]


class PostprocessTests(unittest.TestCase):
    def fixture(self):
        path = Path(__file__).with_name('postprocess_blender.py')
        tree = ast.parse(path.read_text())
        function = next(node for node in tree.body
                        if isinstance(node, ast.FunctionDef) and node.name == 'transform_object')
        namespace = dict(transform_point=transform_point, mouth_delta=mouth_delta,
                         inverse_transpose_normal=inverse_transpose_normal,
                         measure_profile=measure_profile,
                         Matrix=SimpleNamespace(Identity=lambda n: 'identity'))
        exec(compile(ast.Module(body=[function], type_ignores=[]), str(path), 'exec'), namespace)
        base = [(.05, -.08, 1.65), (-.05, -.07, 1.6)]
        delta = (.001, -.002, -.003)
        opened = [tuple(a + b for a, b in zip(p, delta)) for p in base]
        vertices = [SimpleNamespace(co=p) for p in base]
        keys = Blocks([SimpleNamespace(name='Basis', data=vertices),
                       SimpleNamespace(name='MouthOpen', data=[SimpleNamespace(co=p) for p in opened])])
        delta_attr = SimpleNamespace(domain='POINT', data=[SimpleNamespace(vector=delta) for p in base])
        normals = [(0., -1., 0.)] * 3
        written = []
        mesh = SimpleNamespace(users=1, vertices=vertices, has_custom_normals=True,
                               corner_normals=[SimpleNamespace(vector=n) for n in normals],
                               shape_keys=SimpleNamespace(key_blocks=keys),
                               attributes={'MouthOpenDelta': delta_attr}, update=lambda: None,
                               polygons=[SimpleNamespace(area=.01)],
                               normals_split_custom_set=written.extend)
        obj = SimpleNamespace(matrix_world='identity', data=mesh)
        return namespace['transform_object'], obj, base, delta, opened, written

    def test_basis_alias_is_transformed_once(self):
        transform, obj, base, delta, opened, written = self.fixture()
        vertices = obj.data.vertices
        keys = obj.data.shape_keys.key_blocks
        delta_attr = obj.data.attributes['MouthOpenDelta']
        normals = [(0., -1., 0.)] * 3
        pivot = (0, 0, 1.641)
        config = PRESETS['soft_wide']
        transform(obj, config, pivot, require_runtime_morph=True)
        self.assertEqual([v.co for v in vertices], [transform_point(p, config, pivot) for p in base])
        self.assertEqual([v.co for v in keys[1].data], [transform_point(p, config, pivot) for p in opened])
        self.assertEqual([d.vector for d in delta_attr.data], [mouth_delta(delta, config)] * 2)
        self.assertEqual(written, [inverse_transpose_normal(n, config) for n in normals])

    def test_editable_lash_without_delta_preserves_absence(self):
        for config in PRESETS.values():
            transform, obj, base, _, _, _ = self.fixture()
            obj.data.shape_keys = None
            obj.data.attributes.clear()
            pivot = (0, 0, 1.641)
            result = transform(obj, config, pivot)
            self.assertFalse(result['mouth_delta_present'])
            self.assertNotIn('MouthOpenDelta', obj.data.attributes)
            self.assertEqual([v.co for v in obj.data.vertices],
                             [transform_point(p, config, pivot) for p in base])

    def test_missing_delta_rejected_with_shape_keys_and_runtime(self):
        for config in PRESETS.values():
            for runtime, keep_keys in [(False, True), (True, True), (True, False)]:
                transform, obj, base, _, _, _ = self.fixture()
                obj.data.attributes.clear()
                if not keep_keys:
                    obj.data.shape_keys = None
                with self.assertRaises(AssertionError):
                    transform(obj, config, (0, 0, 1.641), require_runtime_morph=runtime)
                self.assertEqual([v.co for v in obj.data.vertices], base)

    def test_runtime_missing_keys_and_wrong_delta_domain_rejected(self):
        for config in PRESETS.values():
            transform, obj, base, _, _, _ = self.fixture()
            obj.data.shape_keys = None
            with self.assertRaises(AssertionError):
                transform(obj, config, (0, 0, 1.641), require_runtime_morph=True)
            obj.data.attributes['MouthOpenDelta'].domain = 'CORNER'
            with self.assertRaises(AssertionError):
                transform(obj, config, (0, 0, 1.641))
            self.assertEqual([v.co for v in obj.data.vertices], base)


if __name__ == '__main__':
    unittest.main()
