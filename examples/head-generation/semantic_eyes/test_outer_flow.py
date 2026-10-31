"""Source chart connectivity contracts without invoking Blender."""
import collections
import unittest
from source_outer_flow import build_outer_source_flow, outer_flow_arc_stations, outer_flow_q
from source_chin_flow import feature_chin_front_strip

LEVELS = [1.544,1.551,1.560,1.570,1.580,1.590,1.599,1.6045,1.608,
          1.614,1.623,1.633,1.644,1.655,1.664,1.673,1.685,1.701,1.720,1.740,1.760,1.780]
LATERAL = [0,.06,.12,.18,.25,.34,.44,.55,.67,.78,.87,.94,1]
NOMINAL = [i/12 for i in range(13)]

def surface(y,q): return (q,0,y)

def boundary(faces):
    counts = collections.Counter(tuple(sorted((a,b))) for f in faces for a,b in zip(f,f[1:]+f[:1]))
    return {edge for edge,n in counts.items() if n==1}, counts

class OuterFlowTest(unittest.TestCase):
    def chart(self):
        sample = lambda y,c: surface(y,outer_flow_q(y,c,LATERAL,NOMINAL))
        vertices = [sample(y,c) for y in LEVELS for c in range(13)]
        faces = []
        for row in range(21):
            if row<3: faces.extend(feature_chin_front_strip(vertices,row,LEVELS,13,sample))
            else: faces.extend((13*row+c,13*row+c+1,13*(row+1)+c+1,13*(row+1)+c) for c in range(12))
        return vertices, faces

    def test_stations_follow_arc_length(self):
        qs,length = outer_flow_arc_stations(1,.94,surface)
        self.assertAlmostEqual(length,.06)
        for actual,expected in zip(qs,[.94,.96,.98,1]): self.assertAlmostEqual(actual,expected)

    def test_conforming_replacement_and_anchors(self):
        v,f = self.chart(); before=list(v); old_boundary,_=boundary(f)
        result,report = build_outer_source_flow(v,f,LEVELS,13,LATERAL,NOMINAL,surface)
        self.assertEqual(v[:len(before)],before)
        self.assertEqual(len(v)-len(before),42)
        self.assertEqual(len(result)-len(f),42)
        new_boundary,counts=boundary(result)
        self.assertEqual(old_boundary,new_boundary)
        self.assertLessEqual(max(counts.values()),2)
        self.assertTrue(all(len(face)==4 for face in result))
        self.assertEqual(len(report['rows']),18)
        self.assertTrue(all(i<len(v) for row in report['rows'] for i in row['vertex_ids']))
        directed=collections.Counter((a,b) for face in result for a,b in zip(face,face[1:]+face[:1]))
        self.assertTrue(all(directed[a,b]==directed[b,a] for (a,b),n in counts.items() if n==2))

    def test_missing_face_fails_closed(self):
        v,f=self.chart(); f=[face for face in f if set(face)!={89,90,103,102}]
        with self.assertRaisesRegex(ValueError,'Incomplete'):
            build_outer_source_flow(v,f,LEVELS,13,LATERAL,NOMINAL,surface)

    def test_wrong_chart_fails_closed(self):
        with self.assertRaisesRegex(ValueError,'versioned'):
            build_outer_source_flow([],[],LEVELS,12,LATERAL,NOMINAL,surface)

if __name__=='__main__': unittest.main()
