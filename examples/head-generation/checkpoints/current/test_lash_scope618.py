import hashlib
from sampling_legacy_contract import legacy_ear_source
import unittest,json
from pathlib import Path
class LashScope618(unittest.TestCase):
 def test_only_declared_style_guides_change(self):
  root=Path(__file__).parent;old=json.loads((root/'baseline_fixtures.json').read_text())['617']['style'];new=json.loads((root/'face_style.json').read_text())
  allowed={'accent_lateral_branch_start','accent_lateral_branch_span','outer_lash_wing_rails','accent_lateral_join_t','accent_lateral_eye_rim_follow','accent_lateral_taper_anchor_profile','accent_lateral_front_bias','preset','accent_lateral_inner_width_profile','accent_lateral_branch_width_H','accent_lateral_guide_inward_power','upper_swept_tuft_outline','lateral_accent_profile','accent_lateral_bottom_t','accent_lateral_center_profile','accent_lateral_branch_separation_H','upper_lash_rails','accent_main_lower_flank_drop_m','accent_main_lower_flank_recede_m','accent_final_crown_relief_m'}
  self.assertEqual({k for k in new if new[k]!=old.get(k)},allowed)
  for name in ['ears.py','ear_silhouette.py','nose_fair.py','nose_refine.py','eye_socket.py','mouth.py','face.py']:
   data=(root/name).read_bytes()
   if name=='ears.py':data=legacy_ear_source(data.decode()).encode()
   self.assertEqual(hashlib.sha256(data).hexdigest(),json.loads((root/'baseline_fixtures.json').read_text())['617']['sha256'][name])
 def test_main_rails_positive_width_and_outer_root_preserved(self):
  s=json.loads(Path(__file__).with_name('face_style.json').read_text());r=s['upper_lash_rails']
  self.assertTrue(all(p[2]>p[1] for p in r[1:-1]));self.assertEqual(r[0][1],r[0][2]);self.assertEqual(r[-1][1],r[-1][2])
  self.assertEqual(next(p for p in r if p[0]==.65)[1],s['outer_lash_wing_rails'][0][1])
