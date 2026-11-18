"""Independent outer-frame chart and transactional source-grid ownership."""
import unittest
import source_feature_flow as old
import source_joint_frame_flow as joint

joint.feature_flow_chart = old.feature_flow_chart
joint.flow_smoothstep = old.flow_smoothstep


class JointFrameTests(unittest.TestCase):
    def fixture(self):
        levels = [1.544,1.551,1.560,1.570,1.580,1.590,1.599,1.6045,
                  1.608,1.614,1.623,1.633,1.644,1.655,1.664,1.673,
                  1.685,1.701,1.720,1.740,1.760,1.780]
        lateral = [0,.05,.1,.18,.25,.34,.44,.55,.67,.78,.87,.94,1]
        vertices = [(q,0,h) for h in levels for q in lateral]+[(7,8,9)]*120
        return levels,lateral,vertices

    def test_chart_pinned_compensation_and_disabled_identity(self):
        for h,q,want in [(1.590,.44,(.431995031814161,1.5886909627813317)),
                         (1.6045,.55,(.477700176829201,1.602746129991726))]:
            for a,b in zip(joint.joint_frame_chart(h,q),want):self.assertAlmostEqual(a,b,places=13)
            self.assertEqual(joint.joint_frame_chart(h,q,0),(q,h))
        for h in (1.544,1.570,1.580,1.614,1.673):
            for q in (0,.18,.44,.78,1):
                self.assertEqual(joint.joint_frame_chart(h,q),old.feature_flow_chart(h,q))

    def test_only_eighteen_nonfunctional_grid_roles_change(self):
        levels,lateral,vertices = self.fixture();before=vertices.copy()
        report=joint.resample_joint_frame(vertices,levels,lateral,13,lambda h,q:(q+.001,0,h))
        changes=[i for i,(a,b) in enumerate(zip(before,vertices)) if a!=b]
        self.assertEqual(len(changes),18)
        self.assertTrue(all(5*13<=i<9*13 for i in changes))
        self.assertEqual(vertices[22*13:],before[22*13:])
        self.assertEqual(vertices[4*13+6],before[4*13+6])
        self.assertEqual(len(report['stations']),18)

    def test_invalid_sample_is_transactional(self):
        levels,lateral,vertices=self.fixture();before=vertices.copy()
        def sample(h,q):return (q+(1 if h==1.608 and q==.67 else .001),0,h)
        with self.assertRaises(ValueError):
            joint.resample_joint_frame(vertices,levels,lateral,13,sample)
        self.assertEqual(vertices,before)


if __name__ == '__main__':unittest.main()
