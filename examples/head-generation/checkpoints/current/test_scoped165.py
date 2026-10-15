import json,math,unittest
from pathlib import Path
from test_style_math import pure_function
C=json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
class Scoped165Tests(unittest.TestCase):
    def test_large_requested_fairing_is_bounded_without_changing_excluded_vertex(self):
        fn,_=pure_function("shell.py","fair_jaw_boundary",{})
        vertices=[(.3,.3,.03),(.31,.6,.09),(.4,.4,.15),(.5,.2,.5)]
        out,report=fn(vertices,[(0,1,2,3)],C,1,0,0)
        self.assertGreater(report["proposed_displacement_m"],C["jaw_border_max_displacement_ratio"])
        self.assertLessEqual(report["maximum_displacement_m"],C["jaw_border_max_displacement_ratio"]+1e-12)
        self.assertEqual(out[3],vertices[3])
        self.assertTrue(all(v[0]==o[0] for v,o in zip(vertices,out)))
    def test_lower_bridge_opposes_both_existing_directed_edges(self):
        fn,_=pure_function("ears.py","lower_ear_bridge",{})
        bridge=fn(0,1,2)
        for side in [-1,1]:
            faces=[(0,1,4),(1,2,3),bridge]
            if side<0:faces=[tuple(reversed(f)) for f in faces]
            uses={}
            for f in faces:
                for a,b in zip(f,f[1:]+f[:1]):uses.setdefault(tuple(sorted((a,b))),[]).append((a,b))
            for edge in [(0,1),(1,2)]:
                self.assertEqual(len(uses[edge]),2)
                self.assertEqual(uses[edge][0],tuple(reversed(uses[edge][1])))
