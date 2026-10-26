"""Keep source relief corrections separate from pigment and the old reference."""
import unittest
import source_lip_relief_flow as flow

class LipReliefFlowTest(unittest.TestCase):
    def test_shoulder_and_shift_are_not_scaled(self):
        flow.mouth_profile_shift = lambda x:.25
        flow.lip_shoulder_support = lambda x,y:2*y
        flow.lip_detail_depth = lambda x,y:10+2*(y-.25)
        self.assertAlmostEqual(flow.source_lip_relief_delta(0,1),-5.5)

    def test_adapter_preserves_reference(self):
        old='    return inherited_base_depth(x, y) + base_delta(x, y, C, curve, DATA)'
        ref='    original = inherited_base_depth(x, y) + (inherited_rim_depth(x, y) - inherited_base_depth(x, y)) * weight'
        result=flow.adapt({'face.py':old+'\n'+ref,'shell.py':'        "semantic_rows": levels,'})
        self.assertIn(ref,result['face.py'])
        self.assertIn(old+' + source_lip_relief_delta(x,y)',result['face.py'])
        self.assertIn('displacement_gate_unchanged',result['shell.py'])

    def test_unknown_source_fails_closed(self):
        with self.assertRaises(ValueError):flow.adapt({'face.py':'different','shell.py':''})

if __name__=='__main__':unittest.main()
