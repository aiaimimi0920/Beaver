"""Bounded numeric eye-family controls; no identity or character-name selector."""

import copy
import math
from contour_warp import upper_shape, upper_shape_derivative

# Multipliers default to one; offsets are fractions of calibrated face height H.
LIMITS = {
    "eye_spacing": (1.0, 0.94, 1.04),
    "eye_width": (1.0, 0.92, 1.08),
    "upper_arch": (1.0, 0.70, 1.40),
    "upper_asymmetry": (0.0, -0.25, 0.25),
    "lower_arch": (1.0, 0.50, 1.12),
    "lower_roundness": (1.0, 1.0, 1.25),
    "lower_horizontal_roundness": (1.0, 1.0, 1.5),
    "lower_shoulder": (0.0, 0.0, 1.0),
    "upper_roundness": (1.0, 1.0, 2.0),
    "upper_horizontal_roundness": (1.0, 0.70, 1.0),
    "corner_slope_offset": (0.0, -0.018, 0.008),
    "eye_height_offset": (0.0, -0.006, 0.006),
    "iris_width": (1.0, 0.90, 1.08),
    "iris_height": (1.0, 0.90, 1.08),
    "iris_vertical_offset": (0.0, -0.01, 0.01),
    "iris_plane_slope_scale": (1.0, 0.85, 1.0),
    "iris_horizontal_offset": (0.0, -0.012, 0.012),
    "ocular_skin_outline_holdout": (0.0, 0.0, 1.0),
    "simple_iris": (0.0, 0.0, 1.0),
    "short_upper_tuft": (0.0, 0.0, 1.0),
    "secondary_lash_scale": (1.0, 0.70, 1.0),
    "brow_height_offset": (0.0, -0.012, 0.012),
    "brow_rise_offset": (0.0, -0.0255, 0.012),
    "brow_thickness": (1.0, 0.70, 1.20),
    "brow_arch": (1.0, 0.5, 1.5),
    "brow_fullness_offset": (0.0, 0.0, 0.25),
    "brow_softness": (0.0, 0.0, 1.0),
    "brow_head_rounding": (0.0, 0.0, 1.0),
    "lash_band_thickness": (1.0, 0.55, 1.15),
    "lash_contour_follow": (0.0, 0.0, 1.0),
    "supplemental_lashes": (1.0, 0.0, 1.0),
    "layered_lashes": (0.0, 0.0, 1.0),
    "upper_lash_layer": (0.0, 0.0, 1.0),
    "lid_fold": (1.0, 0.0, 1.0),
}


def validate(request):
    if not isinstance(request, dict) or set(request) - LIMITS.keys():
        raise ValueError("Expected only documented numeric eye-family controls")
    result = {key: limits[0] for key, limits in LIMITS.items()}
    for key, value in request.items():
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
        ):
            raise ValueError("Expected finite number: " + key)
        if not LIMITS[key][1] <= value <= LIMITS[key][2]:
            raise ValueError("Outside bounded eye-family range: " + key)
        if key in (
            "supplemental_lashes",
            "layered_lashes",
            "upper_lash_layer",
            "lid_fold",
            "simple_iris",
            "short_upper_tuft",
            "ocular_skin_outline_holdout",
        ) and value not in (
            0,
            1,
        ):
            raise ValueError("Layer control must be numeric zero or one: " + key)
        result[key] = float(value)
    if result["short_upper_tuft"] and (
        result["supplemental_lashes"] != 0 or result["lash_contour_follow"] != 1
    ):
        raise ValueError(
            "Short upper tuft requires isolated layers and full contour following"
        )
    if result["layered_lashes"] and not result["short_upper_tuft"]:
        raise ValueError("Layered lashes require the calibrated short upper tuft")
    if result["upper_lash_layer"] and not result["layered_lashes"]:
        raise ValueError("Upper lash layer requires the calibrated layered lashes")
    if result["secondary_lash_scale"] != 1 and not result["short_upper_tuft"]:
        raise ValueError("Secondary lash scale requires the isolated short upper tuft")
    return result


def upper_change(t, calibration, parameters):
    arch = (
        max(0.0, 1 - abs(t) ** calibration["eye_horizontal_power"])
        ** calibration["eye_upper_power"]
    )
    new_arch = max(
        0.0,
        1
        - abs(t)
        ** (
            calibration["eye_horizontal_power"]
            * parameters["upper_horizontal_roundness"]
        ),
    ) ** (calibration["eye_upper_power"] * parameters["upper_roundness"])
    if parameters["upper_asymmetry"]:
        shaped = dict(
            calibration,
            eye_horizontal_power=calibration["eye_horizontal_power"]
            * parameters["upper_horizontal_roundness"],
            eye_upper_power=calibration["eye_upper_power"]
            * parameters["upper_roundness"],
            eye_upper_asymmetry=parameters["upper_asymmetry"],
        )
        new_arch = upper_shape(t, shaped)
    return calibration["eye_upper_arch"] * (parameters["upper_arch"] * new_arch - arch)


def adapt_style(original, request):
    p = validate(request)
    style = copy.deepcopy(original)
    c, old = style["calibration"], original["calibration"]
    if p["ocular_skin_outline_holdout"]:
        style["ocular_skin_outline_holdout"] = True
    else:
        style.pop("ocular_skin_outline_holdout", None)
    # Preserve the exact neutral parameter snapshot.
    if p == validate({}) and not style.get("illustrated_eye_art", False):
        return style
    old_center = old["eye_center_x"]
    c["eye_center_x"] *= p["eye_spacing"]
    c["eye_half_width"] *= p["eye_width"]
    c["eye_corner_y"] += p["eye_height_offset"]
    c["eye_corner_slope"] += p["corner_slope_offset"]
    if p["upper_asymmetry"]:
        if abs(c["eye_corner_slope"]) > 1e-12:
            raise ValueError(
                "Upper asymmetry requires level eye corners for the outer inverse"
            )
        c["eye_upper_asymmetry"] = p["upper_asymmetry"]
    c["eye_upper_arch"] *= p["upper_arch"]
    c["eye_upper_power"] *= p["upper_roundness"]
    c["eye_horizontal_power"] *= p["upper_horizontal_roundness"]
    c["eye_lower_arch"] *= p["lower_arch"]
    c["eye_lower_power"] *= p["lower_roundness"]
    c["eye_lower_horizontal_power"] *= p["lower_horizontal_roundness"]
    if p["lower_shoulder"]:
        if abs(c["eye_corner_slope"]) > 1e-12:
            raise ValueError("Lower shoulder requires level eye corners")
        c["eye_lower_shoulder"] = p["lower_shoulder"]
    if p["simple_iris"]:
        style["iris_graphic_disc_strength"] = 0.0
        style["iris_accent_strength"] = 0.0
        c["pupil_half_height"] *= 0.84
        c["catchlight_half_width"] *= 0.80
        c["catchlight_half_height"] *= 0.80
    # Iris and brows follow the same eye-local horizontal frame.
    for key in ("iris_center_x", "brow_inner_x", "brow_outer_x"):
        c[key] = c["eye_center_x"] + (old[key] - old_center) * p["eye_width"]
    # The signed side is applied by the eye geometry: negative moves both nasally.
    c["iris_center_x"] += p["iris_horizontal_offset"]
    # Pivot only the authored horizontal support plane about the iris center.
    c["iris_plane_slope"] *= p["iris_plane_slope_scale"]
    c["iris_center_y"] += p["eye_height_offset"] + p["iris_vertical_offset"]
    c["iris_half_width"] *= p["iris_width"]
    c["iris_half_height"] *= p["iris_height"]
    c["brow_height"] += p["eye_height_offset"] + p["brow_height_offset"]
    c["brow_rise"] += p["brow_rise_offset"]
    c["brow_thickness"] *= p["brow_thickness"]
    style["brow_arch_m"] *= p["brow_arch"]
    c["brow_inner_fullness"] += p["brow_fullness_offset"]
    if p["brow_head_rounding"]:
        c["brow_head_rounding"] = p["brow_head_rounding"]
    if p["brow_softness"]:
        # Keep the authored hue; lighten the independent eyebrow only.
        amount = 0.12 * p["brow_softness"]
        style["brow_rgb"] = [
            value + (1 - value) * amount for value in style["brow_rgb"]
        ]
    for field, key in [
        ("eye_center_x_m", "eye_center_x"),
        ("eye_half_width_m", "eye_half_width"),
        ("iris_half_width_m", "iris_half_width"),
    ]:
        style[field] = c["height_m"] * c[key]

    def guide_delta(t):
        # Existing lash trajectory applies a reduced corner slope. Compensate
        # the changed slope only, preserving its original calibrated offset.
        return upper_change(t, old, p) + p["corner_slope_offset"] * t * (
            1 - style["accent_trajectory_slope_scale"]
        )

    for key in ("upper_lash_rails", "outer_lash_wing_rails"):
        for row in style[key]:
            t, lower, upper = row
            row[1] = lower + guide_delta(t)
            row[2] = row[1] + (upper - lower) * p["lash_band_thickness"]
    for key in ("upper_swept_tuft_outline", "inner_lash_fork_outline"):
        for row in style[key]:
            row[1] += guide_delta(row[0])
    for row in style["lid_fold_height_profile"]:
        t = -0.76 + 1.52 * row[0]
        row[1] += (
            p["eye_height_offset"]
            + upper_change(t, old, p)
            + p["corner_slope_offset"] * t
        )
    # A coherent thin band follows the adapted aperture instead of inheriting
    # the original unrelated hand-authored rise profile. Blend zero is identity.
    if p["lash_contour_follow"]:
        rails = []
        tail_end = (
            1.18
            if p["layered_lashes"]
            else 1.025 if style.get("illustrated_eye_art", False) else 1.12
        )
        for i in range(21):
            rail_span = tail_end + 0.98
            t = -0.98 + rail_span * i / 20
            q = i / 20
            join_t = 0.96 if style.get("illustrated_eye_art", False) else 0.75
            tc = min(t, join_t)
            arch = (
                max(0.0, 1 - abs(tc) ** c["eye_horizontal_power"])
                ** c["eye_upper_power"]
            )
            if p["upper_asymmetry"]:
                arch = upper_shape(tc, c)
            center = c["eye_upper_arch"] * arch + c["eye_corner_slope"] * tc * (
                1 - style["accent_trajectory_slope_scale"]
            )
            if t > join_t:
                span = tail_end - join_t
                u = (t - join_t) / span
                h = c["eye_horizontal_power"]
                v = c["eye_upper_power"]
                slope = -c["eye_upper_arch"] * v * h * join_t ** (h - 1) * (
                    1 - join_t**h
                ) ** (v - 1) + c["eye_corner_slope"] * (
                    1 - style["accent_trajectory_slope_scale"]
                )
                if p["upper_asymmetry"]:
                    slope = c["eye_upper_arch"] * upper_shape_derivative(join_t, c) + c[
                        "eye_corner_slope"
                    ] * (1 - style["accent_trajectory_slope_scale"])
                tail_lift = 0.012 if style.get("illustrated_eye_art", False) else 0.020
                end = tail_lift + c["eye_corner_slope"] * tail_end * (
                    1 - style["accent_trajectory_slope_scale"]
                )
                center = (
                    (2 * u**3 - 3 * u * u + 1) * center
                    + (u**3 - 2 * u * u + u) * span * slope
                    + (-2 * u**3 + 3 * u * u) * end
                    + (u**3 - u * u) * span * (-0.012)
                )
            center -= 0.002 * math.sin(math.pi * q)
            width = (
                0.018
                * p["lash_band_thickness"]
                * math.sin(math.pi * q) ** 0.8
                * (0.45 + 0.90 * q)
            )
            if i in (0, 20):
                width = 0.0
            rails.append([t, center - width * 0.55, center + width * 0.45])

        # The nonzero mode is deliberately a single new coherent contour.
        # Fractional mode blends sampled old rails using linear interpolation.
        def sample(rows, t, lane):
            if t <= rows[0][0]:
                return rows[0][lane]
            if t >= rows[-1][0]:
                return rows[-1][lane]
            for a, b in zip(rows, rows[1:]):
                if a[0] <= t <= b[0]:
                    q = (t - a[0]) / (b[0] - a[0])
                    return a[lane] * (1 - q) + b[lane] * q

        old_rails = style["upper_lash_rails"]
        amount = p["lash_contour_follow"]
        style["upper_lash_rails"] = [
            [
                t,
                *[
                    sample(old_rails, t, k) * (1 - amount) + r[k] * amount
                    for k in (1, 2)
                ],
            ]
            for r in rails
            for t in [r[0]]
        ]
    if p["short_upper_tuft"]:
        # The control is safe independently of the optional facial art profile.
        style.update(
            accent_tuft_flare_m=0.0006,
            accent_tuft_free_bow_m=0.0003,
            accent_tuft_rear_scoop_m=0.0004,
            accent_tuft_root_cover_m=0.00015,
            accent_tuft_mid_recede_m=0.00015,
        )
        rails = style["upper_lash_rails"]

        def mid(t):
            return (sample(rails, t, 1) + sample(rails, t, 2)) / 2

        style["upper_swept_tuft_outline"] = [
            [0.42, mid(0.42)],
            [0.55, mid(0.55) + 0.002],
            [0.70, sample(rails, 0.55, 2) + 0.003],
            [0.95, sample(rails, 0.55, 2) + 0.012],
            [0.83, sample(rails, 0.55, 2) - 0.004],
            [0.60, mid(0.60) - 0.001],
        ]
    if style.get("illustrated_eye_art", False):
        # Preserve the aperture; vary stroke weight along its continuous path.
        rails = style["upper_lash_rails"]
        for row in rails:
            t, lo, hi = row
            q = max(0.0, min(1.0, (t + 0.98) / 2.10))
            width = (hi - lo) * (0.48 + 0.90 * q)
            row[1], row[2] = lo, lo + width
        if p["short_upper_tuft"]:

            def top(t):
                return sample(rails, t, 2)

            def middle(t):
                return (sample(rails, t, 1) + top(t)) / 2

            # A low, swept child blade; root remains embedded in the parent.
            style["upper_swept_tuft_outline"] = [
                [0.42, middle(0.42)],
                [0.56, top(0.56) - 0.002],
                [0.73, top(0.73) + 0.002],
                [0.94, top(0.78) + 0.005],
                [0.81, middle(0.81)],
                [0.60, middle(0.60) - 0.001],
            ]
    if p["short_upper_tuft"] and p["secondary_lash_scale"] != 1:
        # Scale the child blade about its embedded root in the parent frame.
        # Keep the full primary band and the eye aperture unchanged.
        amount = p["secondary_lash_scale"]
        rails = style["upper_lash_rails"]

        def parent_mid(t):
            return (sample(rails, t, 1) + sample(rails, t, 2)) / 2

        root = style["upper_swept_tuft_outline"][0][0]
        style["upper_swept_tuft_outline"] = [
            [
                root + (t - root) * amount,
                parent_mid(root + (t - root) * amount) + (y - parent_mid(t)) * amount,
            ]
            for t, y in style["upper_swept_tuft_outline"]
        ]
    if p["layered_lashes"]:
        # Keep the lower side accent subordinate to the primary lateral leaf.
        style["lateral_accent_profile"][6][1] *= 0.45
        style["accent_lower_tip_t"] = 0.40
        rails = style["upper_lash_rails"]

        def lash_mid(t):
            return (sample(rails, t, 1) + sample(rails, t, 2)) / 2

        if p["upper_lash_layer"]:
            # A swept intermediate sheet between the micro tip and outer fork.
            # Reuse its independent mesh, not its former medial placement.
            upper, lower = [], []
            for index in range(17):
                u = index / 16
                t = 0.36 + 0.40 * u
                center = sample(rails, t, 2) + 0.021 * u * u - 0.006 * (1 - u) ** 2
                half_width = 0.004 * math.sin(math.pi * u)
                upper.append([t, center + half_width])
                lower.append([t, center - half_width])
            # Smooth, narrow lens with closed endpoints; do not widen its root
            # into another dark block on the central upper eyelid.
            style["inner_lash_fork_outline"] = upper + list(reversed(lower[1:-1]))

        # Distribute the small upper blade medially; reserve the outer fork
        # for the independent wing rather than stacking all tips at one corner.
        style["upper_swept_tuft_outline"] = [
            [t - 0.55, lash_mid(t - 0.55) + y - lash_mid(t)]
            for t, y in style["upper_swept_tuft_outline"]
        ]
        wing = style["outer_lash_wing_rails"]
        start, end = wing[0][0], wing[-1][0]
        shift = lash_mid(start) - wing[0][1]
        for row in wing:
            weight = (1 - (row[0] - start) / (end - start)) ** 2
            row[1] += shift * weight
            row[2] += shift * weight
        # Shorter rising upper leaf above the longer parent tail.
        for row in wing:
            u = (row[0] - start) / (end - start)
            row[0] = start + (1.10 - start) * u
            lift = 0.022 * u * u * (3 - 2 * u)
            center = (row[1] + row[2]) * 0.5 + lift
            half_width = (row[2] - row[1]) * 0.5
            row[1], row[2] = center - half_width, center + half_width
    # Geometry must remain within the fixed inherited feature-patch footprint.
    low = c["origin_y_m"] + c["height_m"] * (
        c["eye_corner_y"] - abs(c["eye_corner_slope"]) - c["eye_lower_arch"]
    )
    high = c["origin_y_m"] + c["height_m"] * (
        c["eye_corner_y"] + abs(c["eye_corner_slope"]) + c["eye_upper_arch"]
    )
    if not 1.614 < low < high < 1.673:
        raise ValueError(
            "Combined eye controls exceed inherited eye patch vertical envelope"
        )
    return style


def adapt_lower_clearance(source, enabled):
    if not enabled:
        return source
    old = "+ abs((y - cy) / (H * arch))"
    new = "+ abs((y - cy) / (H * arch)) ** (1.0 / power if y < cy else 1.0)"
    if source.count(old) != 1:
        raise ValueError("Inherited pocket clearance radius contract changed")
    return source.replace(old, new, 1)


def adapt_brow_source(source, amount):
    """Round the medial cap without moving the existing eyebrow centerline."""
    if amount == 0:
        return source
    old = "    return original * 0.5 + (lane - 0.5) * original * factor"
    new = '    width = H * C["brow_thickness"] * sin(pi * t) ** (\n        0.7 - 0.3 * C["brow_head_rounding"] * (1 - t) ** 3\n    )\n    return original * 0.5 + (lane - 0.5) * width * factor'
    if source.count(old) != 1:
        raise ValueError("Inherited eyebrow width contract changed")
    return source.replace(old, new, 1)
