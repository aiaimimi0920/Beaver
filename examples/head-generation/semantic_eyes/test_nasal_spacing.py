"""Fail-closed parameter, ownership and paired shape checks without Blender."""
import unittest
import numpy as np
from nasal_spacing_rule import propose_nasal_bridge_spacing
from nasal_spacing_shape import guard_nasal_spacing_shape
from source_polygon_validity import polygon_diagonals
from source_nasal_spacing import adapt


class NasalSpacingContractTests(unittest.TestCase):
    def test_invalid_strength_fails_before_source_access(self):
        for value in (float('nan'), float('inf'), -.01, 1.01):
            with self.assertRaisesRegex(ValueError, 'strength'):
                propose_nasal_bridge_spacing(None, None, None, None, None, strength=value)

    def test_nonfinite_current_source_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Nonfinite'):
            propose_nasal_bridge_spacing([[0, float('nan'), 0]], [], [], [], [])

    def test_incomplete_five_rail_profile_rejected(self):
        with self.assertRaisesRegex(ValueError, 'five-rail'):
            propose_nasal_bridge_spacing(np.zeros((10, 3)), [], [], [[0] * 10] * 4, [])

    def test_skin_cannot_be_declared_ear_ownership(self):
        with self.assertRaisesRegex(ValueError, 'ownership'):
            propose_nasal_bridge_spacing(np.zeros((10, 3)), [], [], [[0] * 10] * 5, [1])

    def test_invalid_handoff_offset_rejected(self):
        for offset in (0, -1, 1.5, 8):
            with self.assertRaisesRegex(ValueError, 'offset'):
                propose_nasal_bridge_spacing(np.zeros((10, 3)), [], [],
                                             [[0] * 10] * 5, [], outer_anchor_offset=offset)

    def test_source_hook_is_after_core_before_chin(self):
        pipeline = ('def run():\n    if True:\n'
                    '        report.update(refine_nasal_source(data))\n'
                    '        from shell import refine_chin_source\n'
                    '        report.update(refine_chin_source(data,report["chin_source_anchors"]))\n')
        result = adapt({'pipeline.py': pipeline, 'shell.py': ''})
        p = result['pipeline.py']
        self.assertLess(p.index('refine_nasal_source(data)'), p.index('refine_nasal_bridge_spacing(data'))
        self.assertLess(p.index('refine_nasal_bridge_spacing(data'), p.index('from shell import refine_chin_source'))
        self.assertNotIn('from chin_source_shape import', result['shell.py'])
        self.assertLess(p.index('report.update(refine_chin_source'), p.index('mouth_context='))

    def test_helper_collision_fails_closed(self):
        pipeline = ('def run():\n    if True:\n'
                    '        report.update(refine_nasal_source(data))\n'
                    '        from shell import refine_chin_source\n'
                    '        report.update(refine_chin_source(data,report["chin_source_anchors"]))\n')
        with self.assertRaisesRegex(ValueError, 'collides'):
            adapt({'pipeline.py': pipeline, 'shell.py': 'def propose_nasal_bridge_spacing(): pass\n'})


class NasalSpacingShapeTests(unittest.TestCase):
    def test_coarse_gate_is_not_accepted(self):
        with self.assertRaisesRegex(ValueError, '32'):
            guard_nasal_spacing_shape(None, None, None, None, None, None, None, None, subdivisions=16)

    def test_exact_plane_and_out_of_bound_offset(self):
        vertices = np.array([[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]])
        faces = [(0, 1, 2, 3)]
        delta = np.zeros_like(vertices)
        report = guard_nasal_spacing_shape(vertices, faces, vertices, faces, [0],
                                           delta, delta, polygon_diagonals)
        self.assertEqual(report['subdivisions'], 32)
        self.assertEqual(len(report['cases']), 4)
        self.assertLess(max(case['max_mm'] for case in report['cases']), 1e-8)
        shifted = vertices + [0, .002, 0]
        with self.assertRaisesRegex(ValueError, '1 mm'):
            guard_nasal_spacing_shape(vertices, faces, shifted, faces, [0],
                                       delta, delta, polygon_diagonals)


if __name__ == '__main__':
    unittest.main()
