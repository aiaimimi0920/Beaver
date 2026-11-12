import unittest
import numpy as np
from forehead_failed_seam_support import forehead_select_failed_seam_supports as choose
class Tests(unittest.TestCase):
 def case(self,**kw):
  def sample(x):
   d=np.zeros((len(x),3));d[:,2]=.0004991-.1*np.abs(x[:,0]);J=np.zeros((len(x),3,2));J[:,2,0]=np.where(x[:,0]<0,.1,-.1)
   return d,J,x[:,0]==0
  args=dict(accepted_mm=[[.01,0]],failed_mm=[[0,0]],original_radii_mm=[[1.,1.]],variable_ids=[29],frozen_field=np.zeros((1,3)),support_ids=[29],guide_paths={},protected_ids=[],sample_field_differential=sample,source_donors=lambda p:[0,1,2,3]if p[0]<0 else[1,4,5,2]);args.update(kw);return choose(**args)
 def test_failed_identifies_accepted_holds(self):
  p=self.case();self.assertEqual(p['held_vertices'],[29]);e=p['evidence'][0];self.assertEqual(e['accepted_parameters_mm'],[.01,0]);self.assertEqual(e['failed_parameters_mm_diagnostic_only'],[0,0]);self.assertEqual(e['frozen_field_reference_m'],[0,0,0]);self.assertEqual(e['probes'][0]['shared_original_edge'],[1,2])
 def test_accepted_feasible_required(self):self.assertEqual(self.case(accepted_mm=[[0,0]])['held_vertices'],[])
 def test_failed_violation_required(self):self.assertEqual(self.case(failed_mm=[[.01,0]])['held_vertices'],[])
 def test_nonguide(self):self.assertEqual(self.case(guide_paths={'blue':[29]})['held_vertices'],[])
 def test_protected(self):self.assertEqual(self.case(protected_ids=[29])['held_vertices'],[])
 def test_reindexed(self):self.assertEqual(self.case(variable_ids=[129],support_ids=[129])['held_vertices'],[129])
 def test_real_shared_edge_required(self):self.assertEqual(self.case(source_donors=lambda p:[0,1,2,3]if p[0]<0 else[4,5,6,7])['held_vertices'],[])
 def test_original_bounds(self):
  with self.assertRaises(ValueError):self.case(original_radii_mm=[[.001,.001]])
 def test_nonfinite(self):
  with self.assertRaises(ValueError):self.case(failed_mm=[[np.nan,0]])
if __name__=='__main__':unittest.main()
