import unittest,copy
import numpy as np
from forehead_paired_stage import forehead_install_paired_stage,ForeheadPairedDiagonalStage
from test_forehead_paired_export import GOOD_POINTS,GOOD_FIELDS,BAD_POINTS,BAD_FIELDS
class Tests(unittest.TestCase):
 def data(self):
  p={'vertices':GOOD_POINTS,'open_delta':GOOD_FIELDS,'faces':[[0,1,2,3]],'affected_new':[0],'topology':{'face_owners':[0]}}
  return p,{'regions':[1],'faces':p['faces']}
 def test_disabled_preserves_zero_rows_and_mandatory_gate(self):
  p,a=self.data();layout={'count':17};g,s=forehead_install_paired_stage(p,a,[0,1,2,3],layout)
  self.assertEqual(layout['count'],17);self.assertEqual(len(s.constraints(GOOD_POINTS,GOOD_FIELDS)),0)
  p.update(vertices=BAD_POINTS,open_delta=BAD_FIELDS)
  with self.assertRaises(ValueError):forehead_install_paired_stage(p,a,[0,1,2,3],layout)
 def test_enabled_is_local_and_detects_actual_axis_failure(self):
  p,a=self.data();layout={'count':17};g,s=forehead_install_paired_stage(p,a,[0,1,2,3],layout,True)
  self.assertEqual(s.count,29);self.assertEqual(layout['paired_diagonal'],(17,46));self.assertGreater(s.constraints(GOOD_POINTS,GOOD_FIELDS).min(),0)
  self.assertLess(s.constraints(BAD_POINTS,BAD_FIELDS).min(),0)
 def test_gradients_converge_away_from_branches(self):
  p,a=self.data();s=ForeheadPairedDiagonalStage(p,a,[0,1,2,3]);v=np.asarray(GOOD_POINTS);d=np.zeros_like(v);d[0,0]=1;h=1e-8
  f=lambda t:s.constraints(v+t*d,GOOD_FIELDS)
  np.testing.assert_allclose((f(h)-f(-h))/(2*h),(f(h/2)-f(-h/2))/h,rtol=2e-5,atol=.002)
 def test_nonboolean_option_rejected(self):
  p,a=self.data()
  with self.assertRaises(ValueError):forehead_install_paired_stage(p,a,[0],{'count':0},'yes')
 def test_no_moving_skin_quads_is_rejected_not_false_success(self):
  p,a=self.data()
  with self.assertRaises(ValueError):forehead_install_paired_stage(p,a,[],{'count':0},True)
if __name__=='__main__':unittest.main()
