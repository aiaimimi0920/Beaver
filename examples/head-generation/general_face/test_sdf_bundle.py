import ast
import unittest
from sdf_bundle import adapted_bake, build_request, NPR
from prompt_contract import select_intent


class SdfBundleTests(unittest.TestCase):
    def test_adaptation_order_axes_and_paths(self):
        config = select_intent({"proportions": {"width": 1.06, "height": .96, "depth": 1.02}})
        source = adapted_bake(config, "assets/aster/general_face_test/")
        self.assertIn('normals /= np.array([1.06, 0.96, 1.02])', source)
        self.assertLess(source.index('normals /= np.array'), source.index('lit = (normals @ light)'))
        self.assertNotIn('head_recovery_662', source)
        self.assertNotIn('head_recovery_664', source)
        self.assertIn('xx, yy = np.meshgrid((u - 0.5) * 0.235, 1.5 + 0.31 * (1 - v))', source)
        ast.parse(source)

    def test_baseline_and_readable_sources(self):
        before = {p.name: p.read_bytes() for p in NPR.glob('*.py')}
        config = select_intent({"preset": "baseline"})
        self.assertNotIn('normals /= np.array', adapted_bake(config, "assets/aster/general_face_test/"))
        request = build_request({"preset": "baseline"}, "assets/aster/general_face_test/")
        compile(request['code'], 'native sdf job', 'exec')
        paths = [o['path'] for o in request['outputs']]
        self.assertIn('assets/aster/general_face_test/sdf/bake.py', paths)
        self.assertIn('assets/aster/general_face_test/sdf/definition_output.py', paths)
        self.assertIn('Geometry/atlas intent mismatch', request['code'])
        self.assertEqual(before, {p.name: p.read_bytes() for p in NPR.glob('*.py')})


if __name__ == '__main__':
    unittest.main()
