"""Graded source topology invariants on deterministic planar fixtures."""
from collections import Counter
import unittest
from source_chin_flow import feature_chin_front_strip, feature_chin_row_columns, feature_chin_side_strip


class ChinFlowTests(unittest.TestCase):
    def test_graded_rows_form_one_quad_disk(self):
        levels = [1.544, 1.551, 1.560, 1.570]
        vertices = [(i, 0, y) for y in levels for i in range(13)]
        faces = []
        for row in range(3):
            faces.extend(feature_chin_front_strip(vertices, row, levels, 13, lambda y, c: (c, 0, y)))
        self.assertEqual([len(feature_chin_row_columns(j,13)) for j in range(4)], [5,7,9,13])
        self.assertEqual(len(faces), 34)
        uses = Counter(tuple(sorted((a,b))) for f in faces for a,b in zip(f,f[1:]+f[:1]))
        self.assertEqual(set(uses.values()), {1,2})
        used = {i for face in faces for i in face}
        self.assertEqual(len(used)-len(uses)+len(faces), 1)
        self.assertTrue(all(len(set(face))==4 for face in faces))
        boundary = {edge for edge,count in uses.items() if count==1}
        self.assertEqual(len(boundary), 22)
        neighbors = {i:set() for i in used}
        for a,b in uses:
            neighbors[a].add(b);neighbors[b].add(a)
        self.assertLessEqual(max(map(len,neighbors.values())), 5)

    def test_lower_mouth_connection_uses_semantic_transition(self):
        levels = [1.544, 1.551, 1.560, 1.570]
        vertices = [(i, 0, y) for y in levels for i in range(13)]
        faces = feature_chin_front_strip(vertices, 2, levels, 13, lambda y,c: (c,0,y))
        left = 55  # second authored transition, after the first three samples
        a, b, c, d, e = 30, 32, 45, 44, 46
        self.assertIn((left,e,c,d), faces)
        self.assertIn((left,d,a,b), faces)
        self.assertNotIn((a,b,c,d), faces)
        self.assertNotIn((e,c,b,left), faces)
        self.assertEqual(vertices[left], (6.8,0,1.562))
        self.assertEqual(len(vertices), 58)
        self.assertEqual(len(faces), 16)
        edges = Counter(tuple(sorted((x,y))) for f in faces for x,y in zip(f,f[1:]+f[:1]))
        self.assertNotIn(tuple(sorted((b,c))), edges)
        self.assertEqual(edges[tuple(sorted((left,d)))], 2)
        self.assertEqual(len({tuple(sorted(f)) for f in faces}), len(faces))

    def test_side_strip_preserves_physical_boundary(self):
        levels = [1.544,1.551,1.560,1.570,1.580]
        vertices = [(i/12,0,y) for y in levels for i in range(13)]
        side = list(range(len(vertices),len(vertices)+5))
        vertices.extend([(1,.001,1.54375),(1,.00708,1.545888),(1,.0236,1.548248),
                         (1,.04484,1.552968),(1,.052628,1.557688)])
        faces, report = feature_chin_side_strip(vertices,levels,13,side)
        self.assertEqual(report['front_rows'], [0,1,2])
        self.assertEqual(len(faces), 6)
        uses=Counter(tuple(sorted((a,b))) for f in faces for a,b in zip(f,f[1:]+f[:1]))
        self.assertEqual(set(uses.values()), {1,2})
        self.assertEqual(sum(count==1 for count in uses.values()), 8)
        for face in faces:
            p=[vertices[i] for i in face]
            for a,b,c in [(0,1,2),(0,2,3),(0,1,3),(1,2,3)]:
                area=(p[b][1]-p[a][1])*(p[c][2]-p[a][2])-(p[b][2]-p[a][2])*(p[c][1]-p[a][1])
                self.assertGreater(area, 0)


if __name__ == '__main__':
    unittest.main()
