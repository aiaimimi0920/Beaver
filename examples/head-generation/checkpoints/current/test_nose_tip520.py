import unittest
from test_style_math import pure_function
from test_scoped_review import C

class NoseTip520(unittest.TestCase):
    def test_local_nose_adjustment_bounded(self):
        curve, _ = pure_function("face.py", "curve", {})
        old = [[.19,0,.045],[.22,.016,.05],[.25,.055,.066],[.28,.0945,.064],[.3,.095,.062],[.34,.038,.041],[.4,0,.038]]
        for i in range(1001):
            v = .19+.21*i/1000
            delta = (curve(v,1,C["nose_stations"])-curve(v,1,old))*.236
            self.assertLess(abs(delta), .002)
            if v >= .34:
                self.assertLess(abs(delta), .00015)
        self.assertGreater(curve(.286,1,C["nose_stations"]),curve(.286,1,old))
        self.assertLess(curve(.25,1,C["nose_stations"]),curve(.25,1,old))
