import unittest
from side_leaf import leaf_sections
class IndependentLeafTests(unittest.TestCase):
 def test_closed_tip_and_embedded_base(self):
  r=leaf_sections(.52,.65,.76,.011,.0025)
  self.assertEqual(r[0][1],r[0][2]);self.assertEqual(r[-1][1],r[-1][2])
  self.assertTrue(all(-0.0025<=lo<=0 and hi>=lo for _,lo,hi in r))
  self.assertAlmostEqual(next(hi for f,lo,hi in r if f==.65),.011)
  self.assertAlmostEqual(r[0][1],0)
  self.assertAlmostEqual(r[-1][1],0)
 def test_reject_invalid_attachment(self):
  with self.assertRaises(AssertionError):leaf_sections(.8,.65,.9,.01,.002)

 def test_new_children_do_not_shift_original_region_ordinals(self):
  from side_leaf import stable_part_order
  originals=[{'name':n} for n in ['shell','eye','ear','bridge']]
  child={'name':'child','side_leaf_parent':'side'}
  self.assertEqual(stable_part_order(originals[:3]+[child]+originals[3:]),originals+[child])
