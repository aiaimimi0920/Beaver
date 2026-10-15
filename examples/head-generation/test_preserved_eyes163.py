"""Owner-approved162 eye parameters must survive the mouth/chin repair."""
import hashlib,json,unittest
from pathlib import Path
class PreservedEyeTests(unittest.TestCase):
    def test_accepted_eye_parameter_fingerprint(self):
        c=json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        keys=['catchlight_grid_resolution', 'catchlight_half_height', 'catchlight_half_width', 'catchlight_offset_x', 'catchlight_offset_y', 'catchlight_surface_offset', 'eye_center_x', 'eye_corner_slope', 'eye_corner_y', 'eye_half_width', 'eye_horizontal_power', 'eye_inner_depth', 'eye_lower_arch', 'eye_lower_horizontal_power', 'eye_lower_power', 'eye_outer_depth', 'eye_rim_vertical_bulge', 'eye_upper_arch', 'eye_upper_power', 'iris_center_x', 'iris_center_y', 'iris_concavity', 'iris_half_height', 'iris_half_width', 'iris_plane_slope', 'iris_radial_rings', 'iris_radial_samples', 'iris_rim_depth', 'iris_vertical_slope', 'orbital_band_width', 'orbital_lateral_fade', 'pocket_body_height_ratio', 'pocket_body_length', 'pocket_body_width_ratio', 'pocket_center_lift', 'pocket_ring_samples', 'pocket_skin_clearance', 'pocket_transition_back', 'pupil_accent_height', 'pupil_accent_width', 'pupil_half_height', 'pupil_half_width', 'pupil_recess']
        digest=hashlib.sha256(json.dumps({k:c[k] for k in keys},sort_keys=True).encode()).hexdigest()
        self.assertEqual(digest,"9617dd62ce081c0343f0b32675a36db7672fec4f80bb29c346203e7d3f1c2ea8")
    def test_chin_follow_remains_small(self):
        c=json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        self.assertEqual(c["chin_follow_down_m"],0.0015)
        self.assertEqual(c["mouth_rest_half_gap"],0.00065)
