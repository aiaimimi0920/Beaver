"""Reindexed saved two-quad regression, no character IDs in production."""
import unittest,copy
import numpy as np
from forehead_paired_stage import ForeheadPairedDiagonalStage
POINTS = [[0.06722347438335419, -0.06939242780208588, 1.6004215478897095], [0.05448220297694206, -0.07462605088949203, 1.5990554094314575], [0.05716882273554802, -0.0736997053027153, 1.5972708463668823], [0.07160818576812744, -0.06188588589429855, 1.5990183353424072], [0.01804177090525627, -0.06538209319114685, 1.5318745374679565], [0.025494949892163277, -0.07367783784866333, 1.5426193475723267], [0.015485684387385845, -0.07674989849328995, 1.5389752388000488], [0.01429278776049614, -0.07507231086492538, 1.5323140621185303]]
FIELDS = [[0.0, 0.0, 1.7679210873033924e-10], [0.0, 0.0, 3.0930089600644806e-09], [0.0, 0.0, 8.846025245645706e-09], [0.0, 0.0, 2.4551489730129106e-09], [0.0, 2.811423627235854e-08, -0.001466091489419341], [0.0, 0.00042910611955448985, -0.0021433690562844276], [0.0, 0.0, -0.0013512514997273684], [0.0, 1.2883358067483641e-05, -0.0014838032657280564]]
class Tests(unittest.TestCase):
 def stage(self):
  p={'vertices':POINTS,'open_delta':FIELDS,'faces':[[0,1,2,3],[4,5,6,7]],'affected_new':[0,1],'topology':{'face_owners':[0,1]}}
  return ForeheadPairedDiagonalStage(p,{'faces':p['faces'],'regions':[1,1]},list(range(8)))
 def test_saved_input_reinitializes_exactly(self):
  s=self.stage();self.assertGreater(s.raw(POINTS,FIELDS).min(),0)
  expected=(np.asarray(POINTS,np.float32)+np.asarray(FIELDS,np.float32)).astype(float)
  np.testing.assert_array_equal(np.asarray(POINTS)+np.asarray(FIELDS)+s.pose_rounding_offset,expected)
  self.assertTrue(s.export_guard.require(POINTS,FIELDS)['passed'])
 def test_old_unanchored_surrogate_reproduces_two_negative_rows(self):
  s=self.stage();s.pose_rounding_offset[:]=0
  rows=s.raw(POINTS,FIELDS).reshape(2,29)
  self.assertLess(rows[0,27],0);self.assertLess(rows[1,26],0)
  self.assertEqual(np.count_nonzero(rows<=0),2)
 def test_offset_constant_and_inputs_untouched(self):
  s=self.stage();offset=s.pose_rounding_offset.copy();p=np.asarray(POINTS);f=np.asarray(FIELDS);oldp=p.copy();oldf=f.copy()
  s.constraints(p,f);np.testing.assert_array_equal(p,oldp);np.testing.assert_array_equal(f,oldf)
  p[0,0]+=1e-7;s.constraints(p,f);np.testing.assert_array_equal(s.pose_rounding_offset,offset)
if __name__=='__main__':unittest.main()
