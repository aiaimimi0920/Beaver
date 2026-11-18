"""Shared corridor routing preserves original boundary geometry and graph validity."""
import unittest
import numpy as np
from forehead_star_topology import ForeheadStarTopology, forehead_guide_corners
from forehead_boundary_route import forehead_boundary_chain, reroute_forehead_boundary_guides


class BoundaryRouteTests(unittest.TestCase):
    def grid(self, compressed=True, permutation=False):
        columns, rows = 9, 8
        points = np.array([[.018*x, -.1, 1.5+.02*z] for z in range(rows) for x in range(columns)])
        for z, offset in ({4: .01, 5: .015} if compressed else {}).items():
            for x in range(columns):
                points[z*columns+x, 2] += offset*(x/(columns-1))**3
        faces = [[z*columns+x, z*columns+x+1, (z+1)*columns+x+1, (z+1)*columns+x]
                 for z in range(rows-1) for x in range(columns-1)]
        paths = {name: list(range(row*columns, (row+1)*columns))
                 for name, row in zip(('red','orange','green','blue','purple'), (6,5,3,2,1))}
        protected = {i*columns for i in range(rows)} | {paths['green'][-1]}
        if permutation:
            order = np.random.default_rng(191).permutation(len(points))
            mapping = np.argsort(order)
            points = points[order]
            faces = [[int(mapping[i]) for i in q] for q in faces]
            paths = {name: [int(mapping[i]) for i in path] for name, path in paths.items()}
            protected = {int(mapping[i]) for i in protected}
        return ForeheadStarTopology(points, faces, [1]*len(faces), paths), protected

    def test_route_preserves_original_points_and_quad_contract(self):
        builder, protected = self.grid()
        old = np.array(builder.points)
        green = builder.paths['green'][-1]
        report = reroute_forehead_boundary_guides(builder, protected, {green})
        result = builder.finish()
        self.assertTrue(report['changed'])
        self.assertTrue(report['boundary_coordinates_preserved'])
        np.testing.assert_array_equal(np.array(result['vertices'])[:len(old)], old)
        self.assertEqual(result['paths']['green'][-1], green)
        self.assertFalse(forehead_guide_corners(result['faces'], result['paths']['orange']))
        self.assertTrue(all(len(face)==4 for face in result['faces']))
        used = set()
        for path in result['paths'].values():
            self.assertFalse(used.intersection(path))
            used.update(path)

    def test_reindexing_keeps_semantic_endpoint(self):
        endpoints = []
        for reindex in (False, True):
            builder, protected = self.grid(permutation=reindex)
            root = builder.paths['green'][-1]
            result = reroute_forehead_boundary_guides(builder, protected, {root})
            builder.finish()
            endpoints.append(builder.points[result['orange']['endpoint']])
        np.testing.assert_allclose(endpoints[0], endpoints[1], atol=1e-10)

    def test_uncompressed_endpoint_is_unchanged(self):
        builder, protected = self.grid(compressed=False)
        before = [list(face) for face in builder.faces]
        result = reroute_forehead_boundary_guides(builder, protected, set())
        self.assertFalse(result['changed'])
        self.assertEqual(builder.faces, before)

    def test_disconnected_boundary_rejected(self):
        with self.assertRaises(ValueError):
            forehead_boundary_chain([(0,1),(2,3)],0,3)


if __name__ == '__main__':
    unittest.main()
