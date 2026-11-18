import unittest
import numpy as np
from forehead_seam_hint import forehead_revalidate_seam_hint as revalidate
class Tests(unittest.TestCase):
 def case(self,**kw):
  def sample(x):
   d=np.zeros((len(x),3));d[:,2]=.0004989;return d,np.zeros((len(x),3,2)),np.zeros(len(x),bool)
  args=dict(previous_selection={'evidence':[{'vertex':8,'columns':[91,92],'accepted_parameters_mm':[0,0],'frozen_field_reference_m':[0,0,0]}]},coordinates_mm=np.zeros((2,2)),variable_ids=[99,8],frozen_field=np.zeros((2,3)),support_ids=[8,99],guide_paths={},protected_ids=[],sample_field_differential=sample,source_identity='author/chart/fields-hash',previous_source_identity='author/chart/fields-hash');args.update(kw);return revalidate(**args)
 def test_remap_by_id(self):self.assertEqual(self.case()['held_columns'],[2,3])
 def test_source_identity(self):self.assertEqual(self.case(previous_source_identity='other')['held_vertices'],[])
 def test_coordinates_exact(self):self.assertEqual(self.case(coordinates_mm=[[0,0],[1e-16,0]])['held_vertices'],[])
 def test_baseline_exact(self):self.assertEqual(self.case(frozen_field=[[0,0,0],[1e-16,0,0]])['held_vertices'],[])
 def test_no_guide_or_protected(self):
  self.assertEqual(self.case(guide_paths={'x':[8]})['held_vertices'],[]);self.assertEqual(self.case(protected_ids=[8])['held_vertices'],[])
 def test_not_permanent_and_inactive_skipped(self):
  self.assertEqual(self.case(support_ids=[99])['held_vertices'],[]);self.assertEqual(self.case(active_slack_mm=.00001)['held_vertices'],[])
if __name__=='__main__':unittest.main()
