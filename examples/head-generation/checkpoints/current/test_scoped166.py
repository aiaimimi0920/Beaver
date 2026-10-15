import json,unittest
from pathlib import Path
from test_style_math import pure_function
S=json.loads(Path(__file__).with_name("face_style.json").read_text());C=S["calibration"]
class MouthPigmentTests(unittest.TestCase):
    def test_local_original_pigment_is_bounded_and_symmetric(self):
        fn,_=pure_function("textures.py","lip_pigment",{"STYLE":S,"C":C,"H":C["height_m"],"Y0":C["origin_y_m"]})
        y=C["origin_y_m"]+C["height_m"]*C["mouth_height"]
        self.assertAlmostEqual(fn(0,y),S["lip_pigment_strength"])
        self.assertEqual(fn(C["height_m"]*C["mouth_half_width"],y),0)
        for x in [0,.003,.01]:self.assertEqual(fn(x,y),fn(-x,y))
        self.assertLess(fn(0,y+.003),1e-20)
    def test_only_rear_return_crease_is_relaxed(self):
        self.assertGreater(C["rear_return_rim_crease"],0)
        self.assertLess(C["rear_return_rim_crease"],1)
        self.assertEqual(C["jaw_normal_boundary_strength"],.4)
