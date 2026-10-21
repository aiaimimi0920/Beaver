"""Behavioral checks for lower-face calibration and preserved upper geometry."""

import copy
import itertools
import json
from pathlib import Path
import unittest
from face_design import apply_design, validate, stations, bezier, LIMITS, researched_underside_lift

HERE = Path(__file__).resolve().parent
BASE = json.loads((HERE.parent / "checkpoints/current/face_style.json").read_text())
PRESETS = json.loads((HERE / "presets.json").read_text())


class FaceDesignTests(unittest.TestCase):
    def test_no_design_is_deep_identity(self):
        out, report = apply_design(BASE, None)
        self.assertEqual(out, BASE)
        self.assertIsNot(out["calibration"], BASE["calibration"])
        self.assertIsNone(report)

    def test_all_authored_control_corners_preserve_upper_face(self):
        old = copy.deepcopy(BASE)
        for preset in PRESETS.values():
            for values in itertools.product(*[(a,(a+b)/2,b) for a,b in LIMITS.values()]):
                with self.subTest(kind=preset["kind"],values=values):
                    design = {**preset, **dict(zip(LIMITS,values))}
                    out, report = apply_design(BASE, design)
                    self.assertLessEqual(report["continuous_half_width_change_bound_m"], .012)
                    self.assertLessEqual(report["front_profile_interpolant_change_bound_m"], .002)
                    self.assertLessEqual(report["chin_return_vertical_displacement_bound_m"], .004)
                    self.assertEqual(bezier(out["calibration"]["width_stations"])[7:],bezier(BASE["calibration"]["width_stations"])[7:])
                    for key in BASE["calibration"]:
                        if key not in {"width_stations","continuous_front_profile","cross_sections","chin_return_lift"}:
                            self.assertEqual(out["calibration"][key],BASE["calibration"][key])
        self.assertEqual(BASE,old)

    def test_presets_have_distinct_width_curves(self):
        profiles = [stations(p) for p in PRESETS.values()]
        self.assertEqual(len({json.dumps(p) for p in profiles}),4)
        compact,soft,straight,blunt = profiles
        self.assertGreater(compact[2][1],soft[2][1])
        self.assertGreater(soft[2][1],straight[2][1])
        self.assertGreater(blunt[0][1],compact[0][1])

    def test_compact_terminal_span_does_not_become_blunt_chin(self):
        # Native candidate 897 exposed a broad flat underside when tilted.
        compact = stations(PRESETS["compact-rounded"])
        self.assertLess(compact[0][1], .04)
        self.assertGreater(compact[2][1], .19)
        out, report = apply_design(BASE, PRESETS["compact-rounded"])
        self.assertGreater(out["calibration"]["researched_chin_underside_lift"], .008)
        self.assertLess(report["chin_return_vertical_displacement_bound_m"], .004)

    def test_underside_lift_starts_at_retained_join(self):
        out, _ = apply_design(BASE, PRESETS["compact-rounded"])
        config = out["calibration"]
        self.assertEqual(researched_underside_lift(config, 0), .008)
        values = [researched_underside_lift(config, i / 1000) for i in range(51)]
        self.assertTrue(all(a <= b for a,b in zip(values, values[1:])))
        self.assertAlmostEqual(values[-1], config["researched_chin_underside_lift"])

    def test_projection_independent_of_taper(self):
        a = PRESETS["soft-tapered"]
        b = {**a,"chin_projection":.006}
        x,_=apply_design(BASE,a);y,_=apply_design(BASE,b)
        self.assertEqual(x["calibration"]["width_stations"],y["calibration"]["width_stations"])
        self.assertNotEqual(x["calibration"]["continuous_front_profile"],y["calibration"]["continuous_front_profile"])

    def test_terminal_roundness_does_not_change_upper_jaw(self):
        a=stations({**PRESETS["soft-tapered"],"chin_roundness":0})
        b=stations({**PRESETS["soft-tapered"],"chin_roundness":1})
        self.assertNotEqual(a[:2],b[:2]);self.assertEqual(a[2:],b[2:])

    def test_invalid_controls_rejected(self):
        for key,(lo,hi) in LIMITS.items():
            for value in (True,None,"0",float('nan'),float('inf'),lo-.01,hi+.01):
                with self.subTest(key=key,value=value):
                    with self.assertRaises(ValueError):validate({**PRESETS['soft-tapered'],key:value})
        for change in ({'version':True},{'version':2},{'kind':'square-universal'},{'length':1}):
            with self.assertRaises(ValueError):validate({**PRESETS['soft-tapered'],**change})

    def test_mismatched_baseline_rejected(self):
        other=copy.deepcopy(BASE);other['calibration']['width_stations'][1][1]+=.001
        with self.assertRaises(ValueError):apply_design(other,PRESETS['soft-tapered'])
        other = copy.deepcopy(BASE)
        other["calibration"]["chin_return_lift"] += .001
        with self.assertRaises(ValueError):
            apply_design(other, PRESETS["compact-rounded"])


if __name__ == '__main__':
    unittest.main()
