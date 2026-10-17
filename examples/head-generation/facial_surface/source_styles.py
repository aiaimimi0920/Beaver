"""Explicit opt-in art profiles; inherited checkpoint stays immutable."""

import json

PROFILE_STYLE = {
    "accent_band_roll_m": 0.0006,
    "accent_cross_arch_ratio": 0.0,
    "accent_cross_section_profile": "rounded",
    "accent_final_crown_relief_m": 0.0005,
    "accent_main_lower_flank_drop_m": 0.0001,
    "accent_main_lower_flank_recede_m": 0.0002,
    "accent_outer_tint_rgb": [0.045, 0.023, 0.035],
    "accent_section_soft_curve": 1.0,
    "brow_arch_m": 0.0018,
    "brow_rgb": [0.055, 0.025, 0.04],
    "iris_crescent_rgb": [0.52, 0.91, 0.69],
    "iris_dark_rgb": [0.025, 0.11, 0.1],
    "iris_light_rgb": [0.28, 0.62, 0.46],
    "iris_rim_rgb": [0.014, 0.04, 0.035],
    "lash_rgb": [0.022, 0.013, 0.022],
    "lower_lash_width_scale": 0.4,
}
PROFILE_CALIBRATION = {
    "iris_concavity": 0.01,
    "iris_rim_depth": -0.0348,
    "pupil_half_height": 0.0243,
    "pupil_half_width": 0.014875000000000001,
}


def apply_style_profile(style, name):
    """Opt-in original eye-art direction; the inherited baseline stays intact."""
    if name is None:
        return json.loads(json.dumps(style))
    if name not in ("clean_emerald", "emerald_layered", "emerald_illustrated"):
        raise ValueError("Unknown face style profile: " + str(name))
    result = json.loads(json.dumps(style))
    result.update(PROFILE_STYLE)
    result["calibration"].update(PROFILE_CALIBRATION)
    if name in ("emerald_layered", "emerald_illustrated"):
        result.update(
            {
                "nose_plane_tint_strength": 0.08,
                "nose_plane_height": 0.05,
                "nose_plane_width": 0.024,
                "accent_tuft_flare_m": 0.0006,
                "accent_tuft_free_bow_m": 0.0003,
                "accent_tuft_rear_scoop_m": 0.0004,
                "accent_tuft_root_cover_m": 0.00015,
                "accent_tuft_mid_recede_m": 0.00015,
            }
        )
        result["calibration"].update(
            {
                "iris_rim_depth": -0.0448,
                "pocket_body_height_ratio": 1.9,
                "pocket_ring_samples": 64,
                "iris_radial_samples": 64,
                "iris_radial_rings": 4,
            }
        )
    if name == "emerald_illustrated":
        result["illustrated_eye_art"] = True
        result["calibration"].update(
            pupil_half_width=0.0115,
            pupil_half_height=0.021,
            catchlight_offset_x=-0.025,
            catchlight_offset_y=0.027,
            catchlight_half_width=0.012,
            catchlight_half_height=0.010,
        )
    return result
