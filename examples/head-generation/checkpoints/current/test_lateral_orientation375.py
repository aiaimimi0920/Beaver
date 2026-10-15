import math,unittest
from lateral_fit import section_rotation,rotate_section
class SectionOrientationTests(unittest.TestCase):
 def test_root_stays_fixed(self):self.assertEqual(section_rotation((1,0,0),(1,-1,0),1,0),0)
 def test_turn_is_bounded_and_mirrored(self):
  a=section_rotation((1,0,0),(1,-1,0),1,.5)
  b=section_rotation((-1,0,0),(-1,-1,0),-1,.5)
  self.assertAlmostEqual(a,-b);self.assertLessEqual(abs(a),math.pi/4)
 def test_rigid_rotation_preserves_section_and_height(self):
  b=(.3,.4,.5);p=(.31,.42,.53);q=rotate_section(p,b,.4)
  self.assertAlmostEqual(math.dist(p,b),math.dist(q,b));self.assertEqual(p[2],q[2])
 def test_aligned_frame_is_unchanged(self):self.assertAlmostEqual(section_rotation((1,1,0),(1,-1,0),1,.5),0)
