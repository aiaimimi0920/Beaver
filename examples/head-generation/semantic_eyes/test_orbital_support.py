"""Small pure mathematical contracts; actual-model evidence is kept separately."""
from pathlib import Path
import sys
import unittest
import numpy as np
sys.path.insert(0, str(Path(__file__).parent))
from orbital_support_rule import orbital_turns, orbital_quad_quality, orbital_zero_blink_guard
from orbital_support_bind import orbital_bind_strip
from nasal_spacing_shape import guard_nasal_spacing_shape


class OrbitalSupportTests(unittest.TestCase):
    def test_turns_and_quad_quality(self):
        v=np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]],float)
        self.assertAlmostEqual(float(orbital_turns(v,[(0,1,2)])[0]),90.)
        a,r=orbital_quad_quality(v,np.array([[0,1,2,3]]));self.assertTrue(np.all(a==90));self.assertEqual(r[0],1.)
        with self.assertRaises(ValueError):orbital_quad_quality(np.zeros((4,3)),np.array([[0,1,2,3]]))
    def test_generated_blink_guard_rejects_entering_support(self):
        old=np.zeros((2,3));new=old.copy();new[0,0]=.1
        self.assertTrue(orbital_zero_blink_guard(old,new,[0],lambda p:0.)['all_moved_stations_generated_zero'])
        for callback in (lambda p:p[0],lambda p:float('nan')):
            with self.assertRaises(ValueError):orbital_zero_blink_guard(old,new,[0],callback)
    def test_missing_semantic_aperture_rejected(self):
        with self.assertRaises(ValueError):orbital_bind_strip(np.zeros((4,3)),[(0,1,2,3)],[1],[],[])
    def test_blink_verification_uses_actual_export_selection_pose(self):
        v=np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]],np.float32)
        zero=np.zeros_like(v);selection=zero.copy();selection[:,1]=.25;seen=[]
        def selector(points,face,pose):seen.append(pose.copy());return {0:1.}
        proof=guard_nasal_spacing_shape(v,[(0,1,2,3)],v,[(0,1,2,3)],{0},zero,zero,selector,
            selector_delta=selection,pose_name='generated blink')
        self.assertTrue(np.array_equal(seen[0],v+selection))
        self.assertIn('generated blink',{case['pose'] for case in proof['cases']})
        with self.assertRaises(ValueError):
            guard_nasal_spacing_shape(v,[(0,1,2,3)],v,[(0,1,2,3)],{0},zero,zero,selector,selector_delta=np.zeros((2,3)))

if __name__ == '__main__':unittest.main()
