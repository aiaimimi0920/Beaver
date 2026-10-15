"""Synthetic CPU raster and packaging checks; no engine invocation."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import numpy as np
from mesh_normals import raster_normals, is_front_facing
from uv_diagnostic import diagnose_triangles
from parameters import validate
from sdf_bundle import adapted_bake, build_request


class SemanticSdfTests(unittest.TestCase):
    def test_real_normal_raster_orientation_and_hole_fill(self):
        # Increasing Blender UV-v raises the Godot normal's y component.
        triangles = [([(0, 0), (1, 0), (1, 1)], [(0, 0, 1), (0, 0, 1), (0, 1, 1)]),
                     ([(0, 0), (1, 1), (0, 1)], [(0, 0, 1), (0, 1, 1), (0, 1, 1)])]
        field, report = raster_normals(triangles, 8)
        self.assertEqual(report['mesh_covered_pixels'], 64)
        self.assertEqual(report['extrapolated_pixels'], 0)
        self.assertGreater(field[0, 3, 1], field[-1, 3, 1])
        np.testing.assert_allclose(np.linalg.norm(field, axis=-1), 1, atol=1e-14)
        field, report = raster_normals([([(0, 0), (1, 0), (.5, 1)], [(1, 0, 0)] * 3)], 16)
        self.assertGreater(report['extrapolated_pixels'], 0)
        np.testing.assert_allclose(field, np.broadcast_to([1, 0, 0], field.shape))
        with self.assertRaises(ValueError):
            raster_normals([], 8)
        with self.assertRaises(ValueError):
            raster_normals([([(0, 0), (1, 0), (1, 1)], [(0, 0, 0)] * 3)], 8)

    def test_reject_tiny_coverage_degeneracy_and_conflicting_overlap(self):
        triangle = ([(0, 0), (1, 0), (1, 1)], [(0, 0, 1)] * 3)
        for bad in [([(.5, .5), (.51, .5), (.5, .51)], [(0, 0, 1)] * 3),
                    ([(0, 0), (0, 0), (0, 0)], [(0, 0, 1)] * 3)]:
            with self.assertRaises(ValueError):
                raster_normals([bad], 256)
        with self.assertRaisesRegex(ValueError, 'diagnostics'):
            raster_normals([triangle, (triangle[0], [(1, 0, 0)] * 3)], 16)
        with self.assertRaisesRegex(ValueError, 'diagnostics'):
            raster_normals([triangle, ([(0, 0)] * 3, [(0, 0, 1)] * 3)], 16)

    def test_diagnostic_separates_projected_overlap_from_uv_collapse(self):
        first = {'triangle_index': 0, 'uv': [(0, 0), (1, 0), (1, 1)],
                 'positions_blender': [(0, 0, 0), (1, 0, 0), (1, 0, 1)],
                 'normals_godot': [(0, 0, 1)] * 3}
        second = {**first, 'triangle_index': 1,
                  'positions_blender': [(0, -.001, 0), (1, -.001, 0), (1, -.001, 1)],
                  'normals_godot': [(1, 0, 0)] * 3}
        collapsed = {**first, 'triangle_index': 2, 'uv': [(0, 0)] * 3}
        result = diagnose_triangles([first, second, collapsed], 8)
        self.assertEqual(result['degenerate_uv_triangles'], 1)
        self.assertGreater(result['degenerate_records'][0]['geometric_twice_area'], 0)
        self.assertGreater(result['conflicting_overlap_samples'], 0)
        conflict = result['conflict_records'][0]
        self.assertAlmostEqual(conflict['forward_depth_difference_m'], .001)
        self.assertAlmostEqual(conflict['position_separation_m'], .001)
        self.assertEqual(conflict['first']['triangle']['triangle_index'], 0)
        self.assertEqual(conflict['second']['triangle']['triangle_index'], 1)

    def test_front_chart_selection_is_geometric_not_normal_averaging(self):
        front = [(0, 0, 0), (1, 0, 0), (1, 0, 1)]
        back = list(reversed(front))
        self.assertTrue(is_front_facing(front))
        self.assertFalse(is_front_facing(back))
        self.assertFalse(is_front_facing([(0, 0, 0), (0, 1, 0), (0, 1, 1)]))
        with self.assertRaises(ValueError):
            is_front_facing([(0, 0, 0)] * 3)
        uv = [(0, 0), (1, 0), (1, 1)]
        candidates = [(front, uv, [(0, 0, 1)] * 3),
                      ([(0, 0, 0), (1, 0, 1), (0, 0, 1)], [(0, 0), (1, 1), (0, 1)], [(0, 0, 1)] * 3),
                      (back, uv, [(1, 0, 0)] * 3)]
        field, report = raster_normals([(u, n) for p, u, n in candidates if is_front_facing(p)], 16)
        self.assertEqual(report['conflicting_overlap_samples'], 0)
        np.testing.assert_allclose(field, np.broadcast_to([0, 0, 1], field.shape))
        # A genuinely front-facing conflicting chart remains a hard failure.
        with self.assertRaisesRegex(ValueError, 'diagnostics'):
            raster_normals([(uv, [(0, 0, 1)] * 3), (uv, [(1, 0, 0)] * 3)], 16)

    def test_package_current_hashes_and_no_proxy(self):
        out = 'assets/aster/general_face_eyes_test/'
        p = validate({})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / out
            target.mkdir(parents=True)
            (root / 'project.godot').write_text('; CPU fixture')
            (target / 'aster_head_runtime_morph.blend').write_bytes(b'CPU fixture')
            (target / 'aster_head.glb').write_bytes(b'CPU fixture')
            (target / 'aster_definition.tres').write_text('[resource]\n')
            (target / 'semantic_parameters.json').write_text(json.dumps(p))
            report = {'semantic_eyes': {'parameters': p}, 'new_glb_sha256': hashlib.sha256(b'CPU fixture').hexdigest()}
            (target / 'generation_report.json').write_text(json.dumps(report))
            receipt = root / 'receipt.json'
            names = ['aster_head_runtime_morph.blend', 'aster_head.glb', 'semantic_parameters.json']
            captured = {'status': 'succeeded', 'jobId': 'fixture-job',
                        'process': {'workspace': str(root), 'scriptSha256': 'fixture-script'},
                        'result': {'success': True, 'exitCode': 0, 'outputs': [
                            {'path': out + name, 'sha256': hashlib.sha256((target / name).read_bytes()).hexdigest()}
                            for name in names]}}
            receipt.write_text(json.dumps({'response': {'result': {'result': captured}}}))
            request = build_request({}, out, root, receipt)
            compile(request['code'], '<native sdf>', 'exec')
            source = adapted_bake(out)
            self.assertNotIn('convex cheek', source)
            self.assertNotIn('from face import', source)
            self.assertIn('from_runtime_blend', source)
            self.assertIn('mesh_normal_report', source)
            self.assertIn('SDF input changed', request['code'])
            for entry in request['outputs']:
                if entry['path'].endswith(('aster_definition.tres', 'generation_report.json')):
                    self.assertIsNotNone(entry['expectedSha256'])
            with self.assertRaisesRegex(ValueError, 'parameter mismatch'):
                build_request({'iris_width': .95}, out, root, receipt)
            (target / 'aster_head.glb').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'receipt output mismatch'):
                build_request({}, out, root, receipt)


if __name__ == '__main__':
    unittest.main()
