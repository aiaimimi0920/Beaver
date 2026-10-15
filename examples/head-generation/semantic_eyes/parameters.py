"""Bounded numeric eye-family controls; no identity or character-name selector."""
import copy
import math

# Multipliers default to one; offsets are fractions of calibrated face height H.
LIMITS = {
    'eye_spacing': (1., .96, 1.04), 'eye_width': (1., .92, 1.08),
    'upper_arch': (1., .70, 1.12), 'lower_arch': (1., .70, 1.12),
    'corner_slope_offset': (0., -.008, .008), 'eye_height_offset': (0., -.006, .006),
    'iris_width': (1., .90, 1.08), 'iris_height': (1., .90, 1.08),
    'brow_height_offset': (0., -.012, .012), 'brow_rise_offset': (0., -.012, .012),
    'brow_thickness': (1., .70, 1.20), 'brow_arch': (1., .5, 1.5),
    'lash_band_thickness': (1., .55, 1.15),
    'supplemental_lashes': (1., 0., 1.), 'lid_fold': (1., 0., 1.),
}


def validate(request):
    if not isinstance(request, dict) or set(request) - LIMITS.keys():
        raise ValueError('Expected only documented numeric eye-family controls')
    result = {key: limits[0] for key, limits in LIMITS.items()}
    for key, value in request.items():
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError('Expected finite number: ' + key)
        if not LIMITS[key][1] <= value <= LIMITS[key][2]:
            raise ValueError('Outside bounded eye-family range: ' + key)
        if key in ('supplemental_lashes', 'lid_fold') and value not in (0, 1):
            raise ValueError('Layer control must be numeric zero or one: ' + key)
        result[key] = float(value)
    return result


def upper_change(t, calibration, parameters):
    arch = max(0., 1 - abs(t) ** calibration['eye_horizontal_power']) ** calibration['eye_upper_power']
    return calibration['eye_upper_arch'] * (parameters['upper_arch'] - 1) * arch


def adapt_style(original, request):
    p = validate(request)
    style = copy.deepcopy(original)
    c, old = style['calibration'], original['calibration']
    # Preserve the exact neutral parameter snapshot.
    if p == validate({}):
        return style
    old_center = old['eye_center_x']
    c['eye_center_x'] *= p['eye_spacing']
    c['eye_half_width'] *= p['eye_width']
    c['eye_corner_y'] += p['eye_height_offset']
    c['eye_corner_slope'] += p['corner_slope_offset']
    c['eye_upper_arch'] *= p['upper_arch']
    c['eye_lower_arch'] *= p['lower_arch']
    # Iris and brows follow the same eye-local horizontal frame.
    for key in ('iris_center_x', 'brow_inner_x', 'brow_outer_x'):
        c[key] = c['eye_center_x'] + (old[key] - old_center) * p['eye_width']
    c['iris_center_y'] += p['eye_height_offset']
    c['iris_half_width'] *= p['iris_width']
    c['iris_half_height'] *= p['iris_height']
    c['brow_height'] += p['eye_height_offset'] + p['brow_height_offset']
    c['brow_rise'] += p['brow_rise_offset']
    c['brow_thickness'] *= p['brow_thickness']
    style['brow_arch_m'] *= p['brow_arch']
    for field, key in [('eye_center_x_m', 'eye_center_x'), ('eye_half_width_m', 'eye_half_width'),
                       ('iris_half_width_m', 'iris_half_width')]:
        style[field] = c['height_m'] * c[key]

    def guide_delta(t):
        # Existing lash trajectory applies a reduced corner slope. Compensate
        # the changed slope only, preserving its original calibrated offset.
        return (upper_change(t, old, p) + p['corner_slope_offset'] * t
                * (1 - style['accent_trajectory_slope_scale']))

    for key in ('upper_lash_rails', 'outer_lash_wing_rails'):
        for row in style[key]:
            t, lower, upper = row
            row[1] = lower + guide_delta(t)
            row[2] = row[1] + (upper - lower) * p['lash_band_thickness']
    for key in ('upper_swept_tuft_outline', 'inner_lash_fork_outline'):
        for row in style[key]:
            row[1] += guide_delta(row[0])
    for row in style['lid_fold_height_profile']:
        t = -.76 + 1.52 * row[0]
        row[1] += p['eye_height_offset'] + upper_change(t, old, p) + p['corner_slope_offset'] * t
    # Geometry must remain within the fixed inherited feature-patch footprint.
    low = c['origin_y_m'] + c['height_m'] * (c['eye_corner_y'] - abs(c['eye_corner_slope']) - c['eye_lower_arch'])
    high = c['origin_y_m'] + c['height_m'] * (c['eye_corner_y'] + abs(c['eye_corner_slope']) + c['eye_upper_arch'])
    if not 1.614 < low < high < 1.673:
        raise ValueError('Combined eye controls exceed inherited eye patch vertical envelope')
    return style
