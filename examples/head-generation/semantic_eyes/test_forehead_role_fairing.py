import unittest,copy
import numpy as np
from forehead_fairing_stage import ForeheadFairingStage
from forehead_fairing_guard import forehead_capture_fairing_guard,forehead_prepare_fairing_stage
from forehead_fairing_guard import forehead_migrate_fairing_guard_v2 as forehead_migrate_v1_to_role_guard
from orbital_support_rule import orbital_turns

class RoleV2Tests(unittest.TestCase):
 def proposal(self):
  p=np.column_stack((np.arange(9)*.01,np.zeros(9),np.zeros(9)));p[4,1]=.005
  paths={'rail':list(range(9))};L=float(np.linalg.norm(np.diff(p,axis=0),axis=1).sum()*1000)
  stage=ForeheadFairingStage(p,paths,{'rail':L});stage.target_energy=stage.initial_energy
  old={'version':1,'topology_identity':'test','reference':stage.reference,'triples':stage.triples.tolist(),'energy_cap':stage.initial_energy,'turn_caps_degrees':orbital_turns(p,stage.triples).tolist()}
  proposal={'vertices':p.tolist(),'paths':paths,'uniform_station_reference_length_mm':{'rail':L},'whole_guide_fairing_reference':stage.reference,'whole_guide_fairing_guard':old,'faces':[]}
  from forehead_fairing_guard import forehead_fairing_topology_identity
  old['topology_identity']=forehead_fairing_topology_identity(proposal)
  return proposal,stage
 def test_migration_retains_normalization_and_records_old_guard(self):
  p,_=self.proposal();old=copy.deepcopy(p['whole_guide_fairing_guard']);new=forehead_migrate_v1_to_role_guard(p)
  self.assertEqual(new['reference'],old['reference']);self.assertEqual(new['retired_pointwise_v1_guard'],old)
  self.assertFalse(new['migration']['normalization_reset']);self.assertEqual(new['version'],2)
 def test_curvature_redistribution_is_explicitly_not_v1(self):
  p,old=self.proposal();points=np.asarray(p['vertices']);q=points.copy();q[:,1]=np.convolve(points[:,1],[.25,.5,.25],mode='same')
  self.assertFalse(old.accepts(q));p['whole_guide_fairing_guard']=forehead_migrate_v1_to_role_guard(p)
  new=forehead_prepare_fairing_stage(p,points);self.assertTrue(new.accepts(q));self.assertFalse(new.evidence(q)['pointwise_v1_admission_claimed'])
 def test_bv_world_gradient(self):
  p,_=self.proposal();p['whole_guide_fairing_guard']=forehead_migrate_v1_to_role_guard(p);s=forehead_prepare_fairing_stage(p,np.asarray(p['vertices']));v,g=s.bv_world(p['vertices'])
  for i,j in ((3,1),(4,0),(4,1),(5,1)):
   a,b=np.asarray(p['vertices']),np.asarray(p['vertices']);a[i,j]+=1e-8;b[i,j]-=1e-8
   fd=(s.bv_world(a)[0]-s.bv_world(b)[0])/2e-8
   np.testing.assert_allclose(fd,g[:,i,j],rtol=1e-6,atol=1e-4)
 def test_separate_bv_cap_cannot_disappear(self):
  p,_=self.proposal();p['whole_guide_fairing_guard']=forehead_migrate_v1_to_role_guard(p);s=forehead_prepare_fairing_stage(p,np.asarray(p['vertices']));s.target_energy=1e9;s.turn_caps[:]=180
  q=np.asarray(p['vertices']);q[4,1]*=1.01;self.assertFalse(s.accepts(q))

if __name__=='__main__':unittest.main()
