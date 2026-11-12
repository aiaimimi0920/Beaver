"""Stored local quad regression; fixture IDs reindexed, no character logic."""
import unittest
from unittest.mock import patch
import numpy as np
from forehead_paired_export import forehead_paired_export_report,make_forehead_paired_export_guard
from orbital_support_rule import orbital_quad_quality
BAD_POINTS = [[0.0684720054268837, -0.06795354932546616, 1.599684238433838], [0.05448220297694206, -0.07462605088949203, 1.5990554094314575], [0.05672191455960274, -0.07385618984699249, 1.5975680351257324], [0.0717775821685791, -0.06140317767858505, 1.598698377609253]]
BAD_FIELDS = [[0.0, 0.0, 7.757066655678102e-10], [0.0, 0.0, 3.0930089600644806e-09], [0.0, 0.0, 7.906420407266523e-09], [0.0, 0.0, 2.640914154028451e-09]]
GOOD_POINTS = [[0.06495560705661774, -0.0705924853682518, 1.6012699604034424], [0.05448220297694206, -0.07462605088949203, 1.5990554094314575], [0.05964485555887222, -0.07280172407627106, 1.5956212282180786], [0.0709812268614769, -0.06365222483873367, 1.600163459777832]]
GOOD_FIELDS = [[0.0, 0.0, 2.894017248067371e-10], [0.0, 0.0, 3.0930089600644806e-09], [0.0, 0.0, 1.3824415390217837e-08], [0.0, 0.0, 1.7831280896274393e-09]]
class PairedExportTests(unittest.TestCase):
 def proposal(self):return {'vertices':GOOD_POINTS,'open_delta':GOOD_FIELDS,'faces':[[0,1,2,3]],'affected_new':[0]}
 def test_real_angle_valid_export_invalid(self):
  angles,ratios=orbital_quad_quality(np.asarray(BAD_POINTS),np.array([[0,1,2,3]]))
  self.assertGreaterEqual(angles.min(),25);self.assertLessEqual(angles.max(),150);self.assertLessEqual(ratios.max(),7)
  self.assertFalse(forehead_paired_export_report(BAD_POINTS,BAD_FIELDS,[[0,1,2,3]],[0])['passed'])
 def test_valid_prefix_and_invalid_start(self):
  guard=make_forehead_paired_export_guard(self.proposal());self.assertTrue(guard.accepts(GOOD_POINTS,[0],np.asarray(GOOD_FIELDS)[:1]))
  p=self.proposal();p.update(vertices=BAD_POINTS,open_delta=BAD_FIELDS)
  with self.assertRaisesRegex(ValueError,'paired-export'):make_forehead_paired_export_guard(p)
 def test_candidate_and_final_both_reject(self):
  guard=make_forehead_paired_export_guard(self.proposal());self.assertFalse(guard.accepts(BAD_POINTS,list(range(4)),BAD_FIELDS))
  with self.assertRaises(ValueError):guard.require(BAD_POINTS,BAD_FIELDS)
 def test_full_current_field_replacement_and_float32_addition(self):
  guard=make_forehead_paired_export_guard(self.proposal());replacement=np.array([[.00000003,0,0]])
  expected=np.asarray(GOOD_FIELDS,dtype=np.float32);expected[1]=replacement[0]
  with patch('forehead_paired_export.polygon_diagonals',return_value={0:(1.,1.)})as check:
   self.assertTrue(guard.accepts(GOOD_POINTS,[1],replacement))
  np.testing.assert_array_equal(check.call_args.args[2],(np.asarray(GOOD_POINTS,dtype=np.float32)+expected).astype(float))
 def test_bad_ownership_and_nonfinite_fail_closed(self):
  for ids in ([True],[1],[0,0],[]):
   with self.assertRaises(ValueError):forehead_paired_export_report(GOOD_POINTS,GOOD_FIELDS,[[0,1,2,3]],ids)
  fields=np.asarray(GOOD_FIELDS);fields[0,0]=np.nan
  with self.assertRaises(ValueError):forehead_paired_export_report(GOOD_POINTS,fields,[[0,1,2,3]],[0])
 def test_sharedfit_invalid_input_before_solver(self):
  from forehead_shared_fit import fit_forehead_shared_constraints
  p=self.proposal();p.update(vertices=BAD_POINTS,open_delta=BAD_FIELDS,paths={'rail':[0,1,2]},topology={'lineage':{},'face_owners':[0]},lineage={})
  with self.assertRaisesRegex(ValueError,'paired-export'):
   fit_forehead_shared_constraints(GOOD_POINTS,GOOD_FIELDS,p,restore=True,physical_stage=True,author_context={'faces':[[0,1,2,3]],'regions':[1]},solver_mode='linearized')
 def test_export_scope_includes_untouched_skin_only(self):
  p=self.proposal();p['faces']=[[0,1,2,3],[0,1,2,3],[0,1,2,3]];p['topology']={'face_owners':[0,1,2]}
  g=make_forehead_paired_export_guard(p,{'faces':p['faces'],'regions':[1,2,10]})
  self.assertEqual(g.face_ids,[0,1])
if __name__=='__main__':unittest.main()
