"""Authored lateral sections translate coherently without reducing mean closure."""
import copy,json,unittest
from types import SimpleNamespace as S
from unittest.mock import patch
import blink_runtime,bundle
from test_blink_ink_columns import FakeMeshObject

class SectionBlinkTests(unittest.TestCase):
    def test_section_mean_side_isolation_and_neutral_preservation(self):
        c=json.loads((bundle.RECIPE/'face_style.json').read_text())['calibration']
        vertices=[];polygons=[];regions=[];sections=[];expected=[]
        for side,ordinal in ((-1,10),(1,11)):
            for station,t in ((1,.65),(2,.8)):
                ids=[];raw=[]
                for rise in (-.004,.001,.006):
                    x=side*c['height_m']*(c['eye_center_x']+c['eye_half_width']*t)
                    y=c['origin_y_m']+c['height_m']*c['eye_corner_y']+rise
                    i=len(vertices);ids.append(i);vertices.append(S(index=i,co=S(x=x,y=.02,z=y-.021)))
                    sections.append(S(value=station));raw.append(blink_runtime.closure_delta(x,y,'lateral',c))
                polygons.append(S(index=len(polygons),vertices=ids));regions.append(S(value=ordinal))
                expected.append((side,ids,sum(raw)/len(raw)))
        obj=FakeMeshObject(vertices,polygons,regions);obj.data.attributes['BlinkSection']=S(data=sections)
        before=copy.deepcopy(vertices)
        with patch.object(blink_runtime,'verify_pose_orientation',return_value={'mock':True}):
            blink_runtime.build_blink_shapes(obj,{'Outer lateral eye liner -1':10,'Outer lateral eye liner 1':11},c)
        self.assertEqual(before,vertices)
        for side,name in ((-1,'blink.L'),(1,'blink.R')):
            for owned,ids,mean in expected:
                for i in ids:self.assertAlmostEqual(obj.keys[name].data[i].co.z-vertices[i].co.z,mean if owned==side else 0,places=12)

if __name__=='__main__':unittest.main()
