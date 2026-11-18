import sys,unittest,copy
from pathlib import Path

import numpy as np
from scipy.optimize import OptimizeResult
from unittest.mock import patch
import forehead_seam_growth as m
class Budget(unittest.TestCase):
 def case(self,success=False):
  start=np.array([1.,2.]);radius=np.ones(2);calls=[];clock=[0.]
  selection={'held_vertices':[7],'held_columns':[0,1],'source_identity':'same','evidence':[{'vertex':7,'columns':[0,1],'accepted_parameters_mm':[1.,2.],'frozen_field_reference_m':[0,0,0],'accepted_field_slack_mm':1e-7}]}
  context={'variable_ids':[7],'frozen_field_reference_m':[[0,0,0]],'support_ids':[7],'protected_ids':[],'guide_paths':{},'source_identity':'same'}
  def solver(x,r,*args,**kw):
   calls.append((x.copy(),r.copy(),kw));clock[0]+=kw['wall_seconds']
   return OptimizeResult(x=x,success=success,nit=24,message='bounded'),{'status':'admitted_checkpoint'if success else 'wall_limit','steps':[]}
  with patch.object(m,'forehead_restore_linearized',solver),patch.object(m.time,'monotonic',lambda:clock[0]):
   result,meta=m.forehead_restore_seam_growth(start,radius,lambda x:np.ones(2),None,None,(0,1),None,selection,None,np.ones(1),wall_seconds=240,initial_held_seconds=180,failed_selection=lambda x:{'held_vertices':[],'held_columns':[],'evidence':[]},active_set_growth=True,growth_context=context)
  return calls,result,meta
 def test_budget_and_original_frames(self):
  calls,r,meta=self.case();self.assertEqual([c[2]['wall_seconds']for c in calls],[180,30]);self.assertFalse(r.success)
  for x,b,k in calls:np.testing.assert_array_equal(x,[1,2]);np.testing.assert_array_equal(b,[1,1]);self.assertEqual(k['max_iterations'],24)
  self.assertEqual([c[2]['temporary_held_columns']for c in calls],[[0,1],[]]);self.assertEqual(meta['total_elapsed_seconds'],210)
 def test_current_success_stops(self):
  calls,r,meta=self.case(True);self.assertEqual(len(calls),1);self.assertTrue(r.success);self.assertEqual(meta['successful_held_columns'],[0,1])
 def test_deadline_bound(self):
  with self.assertRaises(ValueError):m.forehead_restore_seam_growth(np.zeros(2),np.ones(2),None,None,None,None,None,{},None,None,wall_seconds=301,active_set_growth=True)
 def test_no_fallback_reserve_rejected(self):
  for value in (0,240,float('nan'),float('inf')):
   with self.assertRaises(ValueError):m.forehead_restore_seam_growth(np.zeros(2),np.ones(2),None,None,None,None,None,{},None,None,wall_seconds=240,initial_held_seconds=value,active_set_growth=True)
 def test_legacy_extended_budget_rejected(self):
  with self.assertRaises(ValueError):m.forehead_restore_seam_growth(np.zeros(2),np.ones(2),None,None,None,None,None,{},None,None,wall_seconds=240)
 def test_pure_validator_rejects_non_scalar_and_boolean(self):
  for value in (True, '240', [240], np.array([240]), float('nan'), float('inf')):
   with self.assertRaises(ValueError):m.forehead_validate_seam_budget(value, None, True)
   with self.assertRaises(ValueError):m.forehead_validate_seam_budget(240, value, True)
  with self.assertRaises(ValueError):m.forehead_validate_seam_budget(120, None, 1)
 def test_pure_validator_defaults_and_extended(self):
  self.assertIsNone(m.forehead_validate_seam_budget(120, None, False))
  self.assertIsNone(m.forehead_validate_seam_budget(240, 180, True))
 def test_shared_entry_rejects_before_geometry(self):
  from forehead_shared_fit import fit_forehead_shared_constraints
  with self.assertRaisesRegex(ValueError,'wall budget'):
   fit_forehead_shared_constraints(None,None,None,seam_wall_seconds=float('nan'))
if __name__=='__main__':unittest.main()
