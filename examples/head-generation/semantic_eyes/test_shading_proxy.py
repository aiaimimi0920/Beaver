"""CPU tests for art-directed policy, actual geometry fit and light conventions."""
import unittest
import hashlib
import json
from pathlib import Path
import tempfile
from receipt_binding import bind_generation_receipt
import numpy as np
from shading_proxy import fit_proxy, proxy_normals
from sdf_bundle import adapted_bake


def fixture():
    u, v = np.meshgrid(np.linspace(.05, .95, 21), np.linspace(.05, .95, 23))
    uv = np.column_stack((u.ravel(), v.ravel()))
    points = np.column_stack(((uv[:, 0] - .5) * .235,
                              -.08 + .005 * np.sin(uv[:, 0] * 40),
                              1.479 + .31 * uv[:, 1]))
    return uv, points


class ShadingProxyTests(unittest.TestCase):
    def test_fit_axis_mirror_and_direction_masks(self):
        uv, points = fixture()
        fit = fit_proxy(uv, points)
        np.testing.assert_allclose(fit['uv2_to_blender_xz'], [[.235, 0], [0, .31], [-.1175, 1.479]], atol=1e-12)
        field = proxy_normals(fit, 64)
        np.testing.assert_allclose(np.linalg.norm(field, axis=-1), 1, atol=1e-14)
        np.testing.assert_allclose(field[..., 0], -field[:, ::-1, 0], atol=1e-12)
        np.testing.assert_allclose(field[..., 1:], field[:, ::-1, 1:], atol=1e-12)
        self.assertTrue((field @ [0, 0, 1] > 0).all())
        self.assertFalse((field @ [0, 0, -1] > 0).any())
        self.assertTrue((field[:, 32:] @ [1, 0, 0] > 0).all())
        self.assertFalse((field[:, :32] @ [1, 0, 0] > 0).any())
        self.assertTrue((field[0] @ [0, 1, 0] > 0).all())
        self.assertTrue((field[-1] @ [0, -1, 0] > 0).all())
        # Every horizontal scanline has at most one shadow transition at yaw60.
        mask = field @ [np.sqrt(3) / 2, 0, .5] > 1e-8
        self.assertTrue((np.count_nonzero(np.diff(mask.astype(int), axis=1), axis=1) <= 1).all())

    def test_current_geometry_fit_and_microrelief_exclusion(self):
        uv, points = fixture()
        fit = fit_proxy(uv, points)
        moved = points.copy(); moved[:, 0] *= 1.06; moved[:, 2] = 1.6 + .96 * (moved[:, 2] - 1.6)
        changed = fit_proxy(uv, moved)
        self.assertAlmostEqual(changed['half_width_m'] / fit['half_width_m'], 1.06)
        self.assertAlmostEqual(changed['height_m'] / fit['height_m'], .96)
        moved = points.copy(); moved[:, 1] += .02 * np.sin(np.arange(len(points)))
        np.testing.assert_array_equal(proxy_normals(fit, 32), proxy_normals(fit_proxy(uv, moved), 32))
        refined = points.copy(); refined[::10, 0] += .0005
        robust = fit_proxy(uv, refined)
        self.assertGreater(robust['local_refinement_outliers'], 0)
        np.testing.assert_allclose(robust['uv2_to_blender_xz'], fit['uv2_to_blender_xz'], atol=1e-12)

    def test_invalid_mapping_fails_without_fallback(self):
        uv, points = fixture()
        for bad_uv, bad_points in [(uv[:3], points[:3]), (uv * .01, points),
                                   (uv[:, ::-1], points), (uv, points * np.nan),
                                   (uv, points * .001)]:
            with self.assertRaises(ValueError):
                fit_proxy(bad_uv, bad_points)
        invalid = points.copy(); invalid[:, 0] += .01 * np.sin(uv[:, 1] * 40)
        with self.assertRaises(ValueError):
            fit_proxy(uv, invalid)

    def test_workspace_copy_requires_opt_in_and_all_original_hashes(self):
        prefix = 'assets/aster/general_face_eyes_test/'
        hashes = {prefix + name: hashlib.sha256(name.encode()).hexdigest() for name in
                  ('aster_head_runtime_morph.blend', 'aster_head.glb', 'semantic_parameters.json')}
        receipt = {'response': {'result': {'result': {
            'status': 'succeeded', 'jobId': 'original-job',
            'process': {'workspace': '/original/workspace', 'scriptSha256': 'original-script'},
            'result': {'success': True, 'exitCode': 0,
                       'outputs': [{'path': p, 'sha256': h} for p, h in hashes.items()]}}}}}
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'receipt.json'
            path.write_text(json.dumps(receipt))
            with self.assertRaisesRegex(ValueError, 'workspace mismatch'):
                bind_generation_receipt(path, temporary, prefix, hashes)
            binding = bind_generation_receipt(path, temporary, prefix, hashes, True)
            self.assertTrue(binding['workspace_copy'])
            self.assertEqual(binding['source_generation_workspace'], '/original/workspace')
            self.assertEqual(binding['current_bake_workspace'], str(Path(temporary).resolve()))
            for key in hashes:
                altered = dict(hashes); altered[key] = 'wrong'
                with self.assertRaisesRegex(ValueError, 'output mismatch'):
                    bind_generation_receipt(path, temporary, prefix, altered, True)

    def test_policy_packaging_preserves_experiment_and_frame_recipe(self):
        out = 'assets/aster/general_face_eyes_test/'
        smooth = adapted_bake(out)
        self.assertIn('from shading_proxy import from_runtime_blend', smooth)
        self.assertIn('not physical mesh normals', smooth)
        self.assertIn('YAW = 24', smooth); self.assertIn('ELEV = 7', smooth)
        self.assertIn('light = np.array([0.0, -1.0 if ei == 0 else 1.0, 0.0])', smooth)
        self.assertIn('atlas[::-1]', smooth)
        self.assertIn('from mesh_normals import from_runtime_blend', adapted_bake(out, 'mesh_normals'))
        with self.assertRaises(ValueError):
            adapted_bake(out, 'unknown')
        compile(smooth, '<proxy bake>', 'exec')


if __name__ == '__main__':
    unittest.main()
