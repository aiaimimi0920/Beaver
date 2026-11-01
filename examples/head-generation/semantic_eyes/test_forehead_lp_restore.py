import unittest
import numpy as np
from forehead_lp_restore import forehead_restore_linearized
class Tests(unittest.TestCase):
 def solve(self,**kw):
  defaults=dict(start=np.array([0.]),radii=np.array([1.]),constraints=lambda x:np.array([x[0]-.5,1.]),jacobian=lambda x:np.array([[1.],[0.]]),admit=lambda x:x[0]>=.5,orientation_rows=(1,2),domain=lambda x:{'valid':True,'reason':None},max_iterations=4,wall_seconds=5.)
  defaults.update(kw);return forehead_restore_linearized(**defaults)
 def test_exact_linear_and_no_model_upgrade(self):
  r,m=self.solve();self.assertTrue(r.success);self.assertAlmostEqual(r.x[0],.5);self.assertFalse(m['full_model_success'])
 def test_measured_quantization_margin_same_bounds(self):
  start=np.array([0.]);radius=np.array([1.]);calls=[]
  def margin(x,c):
   calls.append(x[0]);return (np.array([.006,0.]),{'quantized_failed':True})if c[0]>=-1e-8 else None
  r,m=self.solve(start=start,radii=radius,admit=lambda x:np.round(x[0],2)>=.51,margin_callback=margin)
  self.assertTrue(r.success);self.assertAlmostEqual(r.x[0],.506);self.assertEqual(len(m['margin_requests']),1);np.testing.assert_array_equal(start,[0]);np.testing.assert_array_equal(radius,[1])
 def test_domain_never_bypassed_by_lp(self):
  r,m=self.solve(domain=lambda x:{'valid':x[0]<.4,'reason':'fixed domain'},max_iterations=2)
  self.assertFalse(r.success);self.assertLess(r.x[0],.4);self.assertTrue(any(t.get('rejected')=='Full source/domain/orientation'for s in m['steps']for t in s['trials']))
 def test_immutable_bad_row_no_false_success(self):
  r,m=self.solve(constraints=lambda x:np.array([-.1,1.]),jacobian=lambda x:np.zeros((2,1)),admit=lambda x:False)
  self.assertFalse(r.success);self.assertEqual(m['status'],'linearized_infeasible_or_limit')
 def test_warm_does_not_recenter_original_bounds(self):
  with self.assertRaisesRegex(ValueError,'ORIGINAL'):self.solve(warm=np.array([1.1]))
  r,m=self.solve(warm=np.array([.4]));self.assertTrue(r.success);self.assertLessEqual(r.x[0],1.)
 def test_margin_validation(self):
  with self.assertRaisesRegex(ValueError,'margin'):self.solve(margin_callback=lambda x,c:(np.array([-.1,0]),{}))
 def test_progress_is_detached(self):
  def attack(data):data['steps'].clear();data['parameters_mm'][0]=999
  r,m=self.solve(progress=attack);self.assertTrue(r.success);self.assertEqual(len(m['steps']),1);self.assertLess(r.x[0],1)
 def test_quantization_adapter_units(self):
  from forehead_lp_restore import forehead_quantization_margin_targets
  target,e=forehead_quantization_margin_targets(np.zeros(2),np.eye(2),np.array([2.,4.]),[0],[1],[149.997],[150.0001775])
  self.assertAlmostEqual(target[1],.0011775/4,places=10);self.assertEqual(len(e['quantized_angle_repairs']),1)
 def test_immutable_quantized_corner_rejected(self):
  from forehead_lp_restore import forehead_quantization_margin_targets
  with self.assertRaisesRegex(ValueError,'no variable'):
   forehead_quantization_margin_targets(np.zeros(2),np.zeros((2,1)),np.ones(2),[0],[1],[149.997],[150.01])
 def test_quantization_invalid_ownership_and_nonfinite_policy(self):
  from forehead_lp_restore import forehead_quantization_margin_targets
  base=dict(residuals=np.zeros(2),jacobian=np.eye(2),raw_per_scaled=np.ones(2),lower_rows=[0],upper_rows=[1],smooth_angles=[149.997],quantized_angles=[150.01])
  for bad in ({'lower_rows':[.5]}, {'upper_rows':[0]}, {'lower_rows':[True]},
              {'jacobian':np.array([[np.nan,0],[0,1]])},
              {'safety_degrees':np.nan}, {'positive_scaled':np.inf}):
   with self.subTest(bad=bad), self.assertRaises(ValueError):
    forehead_quantization_margin_targets(**dict(base,**bad))
if __name__=='__main__':unittest.main()
