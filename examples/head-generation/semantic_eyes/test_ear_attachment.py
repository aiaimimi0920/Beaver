"""Upper attachment regression; native mesh and visual review remain separate."""
import copy
import json
from pathlib import Path
import unittest

import numpy as np


HERE = Path(__file__).resolve().parent


class EarAttachmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = json.loads((HERE / 'ear_attachment_fixture.json').read_text())
        cls.results = []
        for side in (-1, 1):
            base = copy.deepcopy(cls.fixture)
            for point in base['vertices']:
                point[0] = side * abs(point[0])
            namespace = {}
            for name in ('source_ear_quad_solver.py', 'ear_flow_geometry.py'):
                exec(compile((HERE / name).read_text(), name, 'exec'), namespace)
            namespace['flow_refine_upper_front'] = lambda result, original: result
            namespace['flow_initialize_ear'] = lambda v, f, b=base: copy.deepcopy(b)
            result = namespace['source_flow_ear_layout'](base['vertices'], [])
            cls.results.append((base, result))

    def test_required_views_and_three_dimensional_corners(self):
        for _, result in self.results:
            report = result['upper_attachment_flow']
            self.assertGreaterEqual(report['minimum_quad_corner_angle'], 20)
            self.assertLessEqual(report['maximum_quad_corner_angle'], 145)
            self.assertGreaterEqual(report['review_minimum_corner_angle'], 5)
            self.assertLessEqual(report['review_maximum_corner_angle'], 160)
            self.assertGreaterEqual(report['minimum_pair_dot'], .49)

    def test_source_boundary_and_quad_connectivity_are_preserved(self):
        for base, result in self.results:
            self.assertEqual(result['vertices'][:40], base['vertices'])
            self.assertEqual(result['quads'][:-3], base['quads'][:-1])
            self.assertEqual(result['quads'][-3:],
                             [[37, 36, 40, 42], [40, 39, 41, 42], [41, 27, 37, 42]])
            self.assertEqual(len(result['vertices']), 43)
            self.assertTrue(all(len(face) == 4 for face in result['quads']))

    def test_split_points_share_convex_source_attribute_weights(self):
        for base, result in self.results:
            vertices = np.asarray(base['vertices'])
            weights = result['authored_vertex_weights']
            for index in range(40):
                self.assertEqual(weights[index], {index: 1.0})
            for index in (40, 41, 42):
                self.assertAlmostEqual(sum(weights[index].values()), 1)
                self.assertTrue(all(value >= 0 for value in weights[index].values()))
            for index in (40, 41):
                expected = sum(vertices[key] * value for key, value in weights[index].items())
                np.testing.assert_allclose(result['vertices'][index], expected, atol=1e-12)
            center = vertices[[37, 36, 39, 27]].mean(axis=0)
            self.assertLessEqual(np.linalg.norm(result['vertices'][42] - center), .001)


if __name__ == '__main__':
    unittest.main()
