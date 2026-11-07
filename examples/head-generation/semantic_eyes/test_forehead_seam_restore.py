import unittest
from unittest.mock import patch
import numpy as np
from scipy.optimize import OptimizeResult
import forehead_seam_restore as module
class Tests(unittest.TestCase):
 def run_case(self,initial=None,callback=None,durations=(20,30),success=(False,True)):
  clock=[0.];calls=[];original=np.array([1.,2.]);radii=np.ones(2);marker=lambda x:np.ones(1)
  def solver(start,radius,*args,**kw):
   k=len(calls);calls.append({'start':start.copy(),'radii':radius.copy(),'held':list(kw['temporary_held_columns']),'budget':kw['wall_seconds'],'constraints':args[0]})
   clock[0]+=durations[k];start[:]=99;radius[:]=99
   return OptimizeResult(x=original+.2,success=success[k],nit=3,message='test'),{'status':'test','steps':[{'trials':[{'SOC':[{},{}]}]}],'elapsed_seconds':durations[k]}
  if callable(callback):
   real=callback
   def callback(x):return real(x,clock)
  with patch.object(module,'forehead_restore_linearized',solver),patch.object(module.time,'monotonic',lambda:clock[0]):
   r,m=module.forehead_restore_seam_stage(original,radii,marker,marker,marker,(0,1),marker,initial or {},None,np.ones(1),failed_selection=callback)
  np.testing.assert_array_equal(original,[1,2]);np.testing.assert_array_equal(radii,[1,1]);self.assertTrue(all(c['constraints']is marker for c in calls))
  return calls,r,m
 def test_held_then_free_original_start(self):
  c,r,m=self.run_case({'held_columns':[0],'held_vertices':[8],'evidence':[]});self.assertEqual([x['held']for x in c],[[0],[]]);self.assertEqual([x['budget']for x in c],[60,100]);self.assertTrue(m['unheld_retry_attempted']);self.assertEqual(m['successful_held_columns'],[])
  for x in c:np.testing.assert_array_equal(x['start'],[1,2])
 def test_free_then_failed_selection_hold_original(self):
  def failed(x,clock):
   np.testing.assert_array_equal(x,[1.2,2.2]);x[:]=888;return {'held_columns':[1],'held_vertices':[9],'evidence':[{'vertex':9}]}
  c,r,m=self.run_case(callback=failed);self.assertEqual([x['held']for x in c],[[],[1]]);np.testing.assert_array_equal(c[1]['start'],[1,2]);self.assertEqual(m['successful_selection']['held_vertices'],[9]);self.assertEqual(m['total_work_iterations'],10);self.assertEqual(m['total_major_iterations'],6);self.assertEqual(m['total_soc_corrections'],4)
 def test_successful_initial_hold_stops(self):
  c,r,m=self.run_case({'held_columns':[0],'held_vertices':[8]},success=(True,));self.assertEqual(len(c),1);self.assertEqual(m['successful_selection']['held_vertices'],[8])
 def test_no_failed_evidence_no_retry(self):
  c,r,m=self.run_case(callback=lambda x,t:{'held_columns':[]},success=(False,));self.assertEqual(len(c),1);self.assertFalse(r.success)
 def test_selector_failure_no_upgrade(self):
  def failed(x,t):raise ValueError('no optional evidence')
  c,r,m=self.run_case(callback=failed,success=(False,));self.assertFalse(r.success);self.assertIn('selection_unavailable',m['failed_seam_selection'])
 def test_deadline_includes_selection(self):
  def failed(x,t):t[0]+=100;return {'held_columns':[0]}
  c,r,m=self.run_case(callback=failed,durations=(30,),success=(False,));self.assertEqual(len(c),1);self.assertFalse(m['failed_guided_held_retry_attempted'])
 def test_positive_targets_retained(self):
  clock=[0.];calls=[];seen=[]
  def margin(x,c):return np.array([.2 if not calls else .1]),{}
  def solver(start,radii,*args,**kw):
   wanted,_=kw['margin_callback'](start,np.ones(1));seen.append(float(wanted[0]));calls.append(1);clock[0]+=1
   return OptimizeResult(x=start,success=len(calls)>1,nit=1,message='x'),{'status':'x','steps':[],'elapsed_seconds':1}
  with patch.object(module,'forehead_restore_linearized',solver),patch.object(module.time,'monotonic',lambda:clock[0]):
   module.forehead_restore_seam_stage(np.zeros(1),np.ones(1),lambda x:np.ones(1),None,None,(0,1),None,{'held_columns':[0]},margin,np.ones(1))
  self.assertEqual(seen,[.2,.2])
 def test_no_alternative_gets_whole_budget(self):
  c,r,m=self.run_case(success=(False,));self.assertEqual(len(c),1);self.assertEqual(c[0]['budget'],120)
if __name__=='__main__':unittest.main()
