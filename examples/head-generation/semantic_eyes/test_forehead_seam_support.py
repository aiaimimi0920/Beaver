import unittest
import numpy as np
from forehead_seam_support import forehead_select_seam_supports as choose,forehead_intersect_held_bounds as bounds
class Tests(unittest.TestCase):
 def case(self,**kw):
  def sample(x):
   slope=np.where(x[:,0]>0,.3,.1);d=np.zeros((len(x),3));d[:,2]=.0004989+slope*x[:,0];J=np.zeros((len(x),3,2));J[:,2,0]=slope
   return d,J,x[:,0]==0
  a=dict(coordinates_mm=np.zeros((2,2)),variable_ids=[8,9],frozen_field=np.zeros((2,3)),support_ids=[8,9],guide_paths={},protected_ids=[],sample_field_differential=sample,source_donors=lambda p:[1,4,5,2]if p[0]>0 else[0,1,2,3]);a.update(kw);return choose(**a)
 def test_selection_and_reindex(self):
  p=self.case();self.assertEqual(p['held_vertices'],[8]);self.assertEqual(p['held_columns'],[0,1]);self.assertEqual(p['eligible_probed_count'],1)
  self.assertEqual(self.case(variable_ids=[108,109],support_ids=[108,109])['held_vertices'],[108])
 def test_pins_guides(self):self.assertEqual(self.case(protected_ids=[8],guide_paths={'x':[9]})['held_vertices'],[])
 def test_infeasible_not_selected(self):self.assertEqual(self.case(frozen_field=np.full((2,3),-.001))['held_vertices'],[])
 def test_donor_change_required(self):self.assertEqual(self.case(source_donors=lambda p:[0,1,2,3])['held_vertices'],[])
 def test_scalar_baseline_tie_not_derivative_owner(self):self.assertEqual(self.case(source_donors=lambda p:[10,11,12,13]if p[0]==0 else[1,4,5,2]if p[0]>0 else[0,1,2,3])['held_vertices'],[8])
 def test_bounded_policy_and_sets(self):
  self.assertEqual(self.case(max_holds=0)['held_vertices'],[]);self.assertEqual(self.case(support_ids={8,9},protected_ids={9})['held_vertices'],[8])
  for kw in ({'max_holds':5},{'probe_mm':np.nan},{'variable_ids':[8,8]}):
   with self.assertRaises(ValueError):self.case(**kw)
 def test_bounds_exact_and_no_mutation(self):
  x=np.array([1.,2.,3.]);lo,hi=bounds(x,np.ones(3),[1]);np.testing.assert_array_equal(lo,[0,2,2]);np.testing.assert_array_equal(hi,[2,2,4]);np.testing.assert_array_equal(x,[1,2,3])
 def test_bounds_reject_malformed(self):
  for h in ([1.,2.],[True],[1,1],[-1],[3]):
   with self.assertRaises(ValueError):bounds([1,2,3],[1,1,1],h)
if __name__=='__main__':unittest.main()
