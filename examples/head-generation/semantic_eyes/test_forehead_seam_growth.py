import copy,unittest
from unittest.mock import patch
import numpy as np
from scipy.optimize import OptimizeResult
import forehead_seam_growth as m
class Tests(unittest.TestCase):
 def setUp(self):
  self.start=np.arange(6,dtype=float);self.ctx={'variable_ids':[10,20,30],'frozen_field_reference_m':np.zeros((3,3)),'support_ids':[10,20,30],'protected_ids':[],'guide_paths':{},'source_identity':'abc'}
 def selection(self,vertices):
  rows=[]
  for v in vertices:
   i=self.ctx['variable_ids'].index(v);rows.append({'vertex':v,'columns':[2*i,2*i+1],'accepted_parameters_mm':self.start[2*i:2*i+2].tolist(),'frozen_field_reference_m':[0,0,0],'accepted_field_slack_mm':1e-7})
  return {'held_vertices':vertices,'held_columns':[x for r in rows for x in r['columns']],'evidence':rows,'source_identity':'abc'}
 def case(self,initial=(),found=((10,),(20,)),success=(False,False,True),callback_cost=0):
  clock=[0.];calls=[];seen=[];margin_targets=[.3,.2,.1]
  def solver(start,radii,*args,**kw):
   k=len(calls);calls.append((start.copy(),radii.copy(),kw));clock[0]+=10
   if kw['margin_callback']:seen.append(kw['margin_callback'](start,np.ones(1))[0][0])
   start[:]=999;radii[:]=999
   return OptimizeResult(x=self.start+.01,success=success[k],nit=2,message='fake'),{'steps':[{'trials':[{'SOC':[{}]}]}],'status':'fake'}
  def failed(x):
   clock[0]+=callback_cost;x[:]=888;return self.selection(list(found[len(calls)-1]))
  def margin(x,c):return np.array([margin_targets[len(calls)-1]]),{}
  with patch.object(m,'forehead_restore_linearized',solver),patch.object(m.time,'monotonic',lambda:clock[0]):
   result,meta=m.forehead_restore_seam_growth(self.start,np.ones(6),lambda x:np.ones(1),None,None,(0,1),None,self.selection(list(initial)),margin,np.ones(1),failed_selection=failed,active_set_growth=True,growth_context=self.ctx)
  return calls,result,meta,seen
 def test_three_legs_union_original_frames(self):
  calls,r,meta,seen=self.case();self.assertEqual([c[2]['temporary_held_columns']for c in calls],[[],[0,1],[0,1,2,3]])
  for s,radius,_ in calls:np.testing.assert_array_equal(s,self.start);np.testing.assert_array_equal(radius,np.ones(6))
  self.assertEqual([c[2]['wall_seconds']for c in calls],[40,55,100]);self.assertEqual(meta['successful_selection']['held_vertices'],[10,20]);self.assertEqual(meta['total_work_iterations'],9);self.assertEqual(seen,[.3,.3,.3])
 def test_free_after_held_can_discover_second_seam_not_repeat_singleton(self):
  calls,r,meta,_=self.case(initial=(10,),found=((),(10,20)))
  self.assertEqual([c[2]['temporary_held_columns']for c in calls],[[0,1],[],[0,1,2,3]])
  self.assertTrue(r.success);self.assertEqual(meta['successful_selection']['held_vertices'],[10,20])
 def test_public_failed_selector_capacity_tracks_opt_in(self):
  import ast,inspect,forehead_shared_fit
  tree=ast.parse(inspect.getsource(forehead_shared_fit.fit_forehead_shared_constraints))
  call=next(n for n in ast.walk(tree)if isinstance(n,ast.Call)and isinstance(n.func,ast.Name)and n.func.id=='forehead_select_failed_seam_supports')
  expr=next(k.value for k in call.keywords if k.arg=='max_holds')
  for enabled,wanted in [(False,1),(True,4)]:
   self.assertEqual(eval(compile(ast.Expression(expr),'<policy>','eval'),{'seam_active_set_growth':enabled}),wanted)
 def test_initial_hold_grows_then_releases(self):
  calls,r,meta,_=self.case(initial=(10,),found=((20,),(20,)))
  self.assertEqual([c[2]['temporary_held_columns']for c in calls],[[0,1],[0,1,2,3],[]]);self.assertEqual(meta['successful_selection']['held_vertices'],[])
 def test_no_repeat_or_false_success(self):
  calls,r,meta,_=self.case(found=((10,),(10,)),success=(False,False));self.assertEqual(len(calls),2);self.assertFalse(r.success);self.assertIsNone(meta['successful_selection'])
 def test_selector_time_charged(self):
  calls,r,meta,_=self.case(callback_cost=115,success=(False,));self.assertEqual(len(calls),1);self.assertFalse(r.success)
 def test_success_stops(self):
  calls,r,meta,_=self.case(initial=(10,),success=(True,));self.assertEqual(len(calls),1)
 def test_wrong_frames_rejected(self):
  for field,value in [('accepted_parameters_mm',[8,9]),('frozen_field_reference_m',[1,0,0]),('accepted_field_slack_mm',-1)]:
   s=self.selection([10]);s['evidence'][0][field]=value
   with self.assertRaises(ValueError):m.forehead_validate_seam_growth_selection(s,self.start,self.ctx)
 def test_duplicate_and_wrong_source(self):
  s=self.selection([10]);s['evidence']*=2
  with self.assertRaises(ValueError):m.forehead_validate_seam_growth_selection(s,self.start,self.ctx)
  s=self.selection([10]);s['source_identity']='other'
  with self.assertRaises(ValueError):m.forehead_validate_seam_growth_selection(s,self.start,self.ctx)
 def test_guide_or_protected_rejected(self):
  for key,value in [('protected_ids',[10]),('guide_paths',{'blue':[10]})]:
   c=copy.deepcopy(self.ctx);c[key]=value
   with self.assertRaises(ValueError):m.forehead_validate_seam_growth_selection(self.selection([10]),self.start,c)
 def test_capacity_release(self):
  clock=[0.];calls=[]
  def solver(s,r,*a,**kw):
   calls.append(kw['temporary_held_columns']);clock[0]+=1
   return OptimizeResult(x=s,nit=1,success=len(calls)==2,message='x'),{'steps':[]}
  with patch.object(m,'forehead_restore_linearized',solver),patch.object(m.time,'monotonic',lambda:clock[0]):
   _,meta=m.forehead_restore_seam_growth(self.start,np.ones(6),None,None,None,(0,1),None,self.selection([10]),None,np.ones(1),failed_selection=lambda x:self.selection([20]),active_set_growth=True,growth_context=self.ctx,max_holds=1)
  self.assertEqual(calls,[[0,1],[]]);self.assertIn('growth_rejected',meta['failed_seam_selection_history'][-1])
 def test_legacy_delegates(self):
  with patch.object(m,'forehead_restore_seam_stage',return_value=('old','meta'))as old:
   self.assertEqual(m.forehead_restore_seam_growth(self.start,np.ones(6),None,None,None,None,None,{},None,None),('old','meta'));old.assert_called_once()
if __name__=='__main__':unittest.main()
