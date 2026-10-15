import json,unittest
from pathlib import Path
from test_style_math import pure_function
class EarReturnTaperTests(unittest.TestCase):
    def test_return_depth_tapers_before_width_collapses(self):
        c=json.loads(Path(__file__).with_name('face_style.json').read_text())['calibration']
        curve,_=pure_function('face.py','curve',{})
        def offset(v):
            w=curve(v,1,c['rear_return_inset_profile'])
            return c['height_m']*c['rear_return_back_offset']*curve(v,1,c['rear_return_depth_weight_profile'])*min(1,w/.01)
        values=[offset(.16+i*.0001) for i in range(1111)]
        self.assertTrue(all(b<=a+1e-12 for a,b in zip(values,values[1:])))
        self.assertLess(offset(.26),.0003)
        self.assertAlmostEqual(offset(.271),0,places=12)
        self.assertGreaterEqual(min(values),-1e-12)
