import json,unittest
from pathlib import Path
from test_style_math import pure_function
class EarRootProfileTests(unittest.TestCase):
    def test_local_reversal_removed_with_bounded_displacement(self):
        root=Path(__file__).parent
        c=json.loads((root/'face_style.json').read_text())['calibration']
        old=json.loads((root/'frozen177-contract.json').read_text())['calibration']['continuous_back_profile']
        curve,_=pure_function('face.py','curve',{})
        samples=[curve(.24+i*.0002,1,c['continuous_back_profile']) for i in range(401)]
        self.assertTrue(all(b<=a+1e-12 for a,b in zip(samples,samples[1:])))
        self.assertGreater(curve(.28,1,old),curve(.26,1,old))
        maximum=0
        for i in range(1001):
            v=i/1000
            d=abs(curve(v,1,c['continuous_back_profile'])-curve(v,1,old))
            maximum=max(maximum,d*c['height_m'])
            if v<=.20 or v>=.38:self.assertLess(d,1e-12)
        self.assertLess(maximum,.0016)
