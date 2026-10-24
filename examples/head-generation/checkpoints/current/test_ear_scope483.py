import hashlib
from sampling_legacy_contract import legacy_shell_source
"""Recovery scope regression against preserved 579 (482 is not in this backup)."""
import json
import unittest
from pathlib import Path

class EarScopeRecovery(unittest.TestCase):
    def test_only_local_fold_implementation_changes(self):
        here = Path(__file__).parent
        baseline = here.parent / 'authoring579'
        fixture = json.loads((here / 'baseline_fixtures.json').read_text())['579']
        old = fixture['style']
        new = json.loads((here / 'face_style.json').read_text())
        self.assertEqual(new["accent_lateral_branch_start"],.04);self.assertEqual(new["accent_lateral_branch_span"],.92)
        new["accent_lateral_branch_start"]=.16;new["accent_lateral_branch_span"]=.76
        new['preset'] = old['preset']
        for key in ['outer_lash_wing_rails','accent_lateral_join_t','accent_lateral_inner_width_profile','accent_lateral_branch_width_H','accent_lateral_guide_inward_power','upper_swept_tuft_outline','lateral_accent_profile','accent_lateral_bottom_t','accent_lateral_center_profile','accent_lateral_branch_separation_H','upper_lash_rails', 'accent_main_lower_flank_drop_m', 'accent_main_lower_flank_recede_m', 'accent_final_crown_relief_m']: new[key] = old[key]
        new.pop('accent_lateral_front_bias',None)
        new.pop('accent_lateral_taper_anchor_profile',None)
        new.pop('accent_lateral_eye_rim_follow',None)
        self.assertEqual(old, new)
        for name in [ 'main_curve.py', 'main_flank.py',
                     'tuft_fit.py', 'eye_socket.py', 'mouth.py', 'face.py', 'shell.py',
                      'ear_shoulder.py', 'ear_surface.py', 'ear_width.py']:
            data = (here / name).read_bytes()
            if name == 'shell.py':
                data = legacy_shell_source(data.decode()).encode()
            self.assertEqual(hashlib.sha256(data).hexdigest(), fixture['sha256'][name])
