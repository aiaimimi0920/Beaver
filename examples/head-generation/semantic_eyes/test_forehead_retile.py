"""Pure contracts for conforming stars and frozen native field staging."""
import unittest
from types import SimpleNamespace
import numpy as np
from forehead_star_topology import (ForeheadStarTopology, forehead_edge_counts,
                                   forehead_guide_corners)
from forehead_retile_fields import stage_forehead_point_fields, forehead_named_field
from forehead_surface_batch import make_forehead_surface_batch
from nasal_source_geometry import AuthorQuadSurface
from feature_flow_recipe import retained_sources


class ForeheadRetileTests(unittest.TestCase):
    def grid(self):
        vertices = np.array([[.1+j*.1, -1., 1.+i*.1] for i in range(5) for j in range(5)])
        faces = [[i*5+j, i*5+j+1, (i+1)*5+j+1, (i+1)*5+j]
                 for i in range(4) for j in range(4)]
        return vertices, faces

    def test_closed_star_removes_corner_without_boundary_splits(self):
        v, faces = self.grid()
        path = [7, 6, 11]
        self.assertEqual(len(forehead_guide_corners(faces, path)), 1)
        builder = ForeheadStarTopology(v, faces, [1]*len(faces), {'test': path})
        builder.refine_star(12)
        result = builder.finish()
        self.assertEqual(len(result['vertices']), len(v)+8)
        self.assertEqual(len(result['faces']), len(faces)+8)
        self.assertEqual(result['euler'], 1)
        np.testing.assert_array_equal(result['vertices'][:len(v)], v)
        self.assertFalse(forehead_guide_corners(result['faces'], result['paths']['test']))
        before, after = forehead_edge_counts(faces), forehead_edge_counts(result['faces'])
        self.assertEqual({e for e, n in before.items() if n == 1},
                         {e for e, n in after.items() if n == 1})
        children = [row for row in result['edge_lineage'] if row['parent'] == [7, 12]]
        self.assertEqual(len(children), 2)

    def test_open_star_is_not_silently_refined(self):
        v, faces = self.grid()
        with self.assertRaisesRegex(ValueError, 'closed'):
            ForeheadStarTopology(v, faces, [1]*len(faces), {}).refine_star(0)

    def test_non_skin_star_rejected(self):
        v, faces = self.grid()
        regions = [1]*len(faces)
        regions[5] = 2
        with self.assertRaises(ValueError):
            ForeheadStarTopology(v, faces, regions, {}).refine_star(12)

    def test_new_fields_use_only_original_snapshots(self):
        layer = SimpleNamespace(name='MouthOpenDelta')
        flag = SimpleNamespace(name='BlinkSection')
        deform = SimpleNamespace(name='groups')
        lineage = {2: {'donors': [0, 1], 'weights': [.25, .75]}}
        fields = [('float_vector', layer, [(1., 2., 3.), (3., 4., 5.)]),
                  ('int', flag, [0, 0])]
        staged, groups, _ = stage_forehead_point_fields(2, lineage, fields,
                                                        [(deform, [{0: 1.}, {0: 1.}])])
        self.assertEqual(forehead_named_field(staged[2], 'MouthOpenDelta'), (2.5, 3.5, 4.5))
        self.assertEqual(groups[2][0][1], {0: 1.})

    def test_layer_wrappers_are_resolved_by_name_not_identity(self):
        first, second = SimpleNamespace(name='MouthOpenDelta'), SimpleNamespace(name='MouthOpenDelta')
        self.assertIsNot(first, second)
        self.assertEqual(forehead_named_field([(first, (1., 2., 3.))], second.name), (1., 2., 3.))
        with self.assertRaises(ValueError):
            forehead_named_field([], second.name)

    def test_discrete_or_deform_boundaries_rejected(self):
        layer = SimpleNamespace(name='BlinkSection')
        lineage = {2: {'donors': [0, 1], 'weights': [.5, .5]}}
        with self.assertRaises(ValueError):
            stage_forehead_point_fields(2, lineage, [('int', layer, [0, 1])], [])
        with self.assertRaises(ValueError):
            stage_forehead_point_fields(2, lineage, [], [(layer, [{0: 1.}, {1: 1.}])])

    def test_negative_or_dependent_donors_rejected(self):
        for row in ({'donors': [0, 1], 'weights': [-.1, 1.]},
                    {'donors': [0, 2], 'weights': [.5, .5]}):
            with self.assertRaises(ValueError):
                stage_forehead_point_fields(2, {2: row}, [], [])

    def test_vectorized_surface_matches_scalar_frozen_quad(self):
        chart = np.array([[0, 0, 0], [2, 0, .2], [1.8, 0, 1.3], [-.1, 0, 1.]])
        vertices = chart.copy()
        vertices[:, 1] = [0, .2, .4, -.1]
        quads = np.array([[0, 1, 2, 3]])
        scalar = AuthorQuadSurface(chart, quads, [1])
        scalar.vertices = vertices.copy()
        batch = make_forehead_surface_batch(chart, vertices, quads)
        weights = np.array([[(1-u)*(1-v), u*(1-v), u*v, (1-u)*v]
                            for u in np.linspace(0, 1, 7) for v in np.linspace(0, 1, 7)])
        xy = weights @ chart[:, [0, 2]]
        expected = [scalar.lineage(*p)[2] for p in xy]
        np.testing.assert_allclose(batch.sample(xy), expected, atol=1e-9)
        self.assertIsNone(batch.sample([[99., 99.]]))

    def test_all_new_modules_remain_readable_sources(self):
        sources = retained_sources()
        for name in ('forehead_star_topology', 'forehead_retile_roles', 'forehead_retile_fair',
                     'forehead_retile_support', 'forehead_retile_mirror', 'forehead_retile_contract',
                     'forehead_retile_fields', 'forehead_retile_shape', 'forehead_retile_apply',
                     'forehead_retile_mesh', 'forehead_surface_batch'):
            self.assertIn(name+'.py', sources)
            compile(sources[name+'.py'], name+'.py', 'exec')


if __name__ == '__main__':
    unittest.main()
