"""Feature-aware source sampling; no late vertex IDs or destination normals."""
import inspect


def flow_smoothstep(value):
    value = max(0.0, min(1.0, value))
    return value * value * (3.0 - 2.0 * value)


def flow_monotone_curve(value, knots, targets):
    """C1 monotone Hermite map; endpoint sampling and ordering stay explicit."""
    if len(knots) != len(targets) or len(knots) < 2:
        raise ValueError('Invalid feature-flow station contract')
    widths = [b - a for a, b in zip(knots, knots[1:])]
    slopes = [(b - a) / width for a, b, width in zip(targets, targets[1:], widths)]
    if min(widths) <= 0 or min(slopes) <= 0:
        raise ValueError('Feature-flow stations reverse their order')
    tangents = [min(1.0, 3.0 * slopes[0])]
    for i in range(1, len(knots) - 1):
        left, right = slopes[i - 1], slopes[i]
        w1, w2 = 2 * widths[i] + widths[i - 1], widths[i] + 2 * widths[i - 1]
        tangents.append((w1 + w2) / (w1 / left + w2 / right))
    tangents.append(min(1.0, 3.0 * slopes[-1]))
    if value <= knots[0]:
        return targets[0] + (value - knots[0]) * tangents[0]
    if value >= knots[-1]:
        return targets[-1] + (value - knots[-1]) * tangents[-1]
    i = next(i for i in range(len(widths)) if knots[i] <= value <= knots[i + 1])
    t = (value - knots[i]) / widths[i]
    return ((2*t**3 - 3*t**2 + 1) * targets[i]
            + (t**3 - 2*t**2 + t) * widths[i] * tangents[i]
            + (-2*t**3 + 3*t**2) * targets[i + 1]
            + (t**3 - t**2) * widths[i] * tangents[i + 1])


def orbital_shell_rounding(height, lateral, mapped_q, mapped_y, strength):
    """Round the shared eye-frame corners with compact C1 source-chart support.

    The true aperture still uses the unmapped reference. The nasal cheek guide
    below 1.608, midline, outer head rim and ear roots remain outside support.
    """
    nx = (lateral-.56)/.38
    ny = (height-1.6435)/.0295
    window = ((1-flow_smoothstep((abs(nx)-1)/.15))
              * (1-flow_smoothstep((abs(ny)-1)/(.4 if ny>0 else .2))))
    amount = .04*strength*window
    return (mapped_q-amount*.38*nx*ny*ny,
            mapped_y-amount*.0295*ny*nx*nx)


def mouth_shell_rounding(height, lateral, mapped_q, mapped_y, strength):
    """Round the oral outer frame and nearby cheek columns, not its aperture."""
    nx=lateral/.44
    ny=(height-1.5845)/.0145
    window=(flow_smoothstep((nx-.5)/.5)
            *(1-flow_smoothstep((nx-1)/.65))
            *(1-flow_smoothstep((abs(ny)-1)/.25)))
    amount=.12*strength*window
    return (mapped_q-amount*.44*nx*ny*ny,
            mapped_y-amount*.0145*ny*nx*nx)


def feature_flow_chart(height, lateral, strength=1.0):
    """Compose two monotone chart maps, preserving the center seam and lower-mouth boundary.

    Geometry is evaluated on the existing character surface at the new chart
    coordinates. Inner aperture samples are captured separately before this map.
    """
    from math import exp
    if not 0 <= lateral <= 1 or not 0 <= strength <= 1:
        raise ValueError('Feature-flow coordinates or strength outside contract')
    if height <= 1.551 or height >= 1.710 or strength == 0:
        return lateral, height
    fade = flow_smoothstep((height - 1.551) / .025) * flow_smoothstep((1.648 - height) / .035)
    q = lateral
    if 0 < lateral < .44:
        t = lateral / .44
        q -= 1.5 * strength * fade * .44 * t*t * (1-t)*(1-t)
    elif lateral > .44:
        t = (lateral - .44) / .56
        q -= 2.0 * strength * fade * .56 * t*t * (1-t)*(1-t)
    # Both ends of the lateral field have zero displacement and zero slope.
    # Broad transverse guide: cheek-side flow rises toward the nose region.
    # Keep the midline tip; carry the guide through the side boundary instead
    # of forcing a U-turn back to its old horizontal row.
    nose = (1-q*q)**2
    cheek = flow_smoothstep(q)
    glabella = exp(-(q / .29)**2) * (1-q*q)**2
    # Preserve the established lip/under-chin chart below the mouth boundary.
    # The new cheek guide must not push outer mouth samples inside its aperture.
    old_nose = exp(-(q / .34)**2) * (1-q*q)**2
    old_knots = [1.551, 1.570, 1.599, 1.614, 1.673, 1.710]
    old_targets = [1.551, 1.570, 1.599 + .008*strength*old_nose,
                   1.614 + .004*strength*old_nose,
                   1.673 - .009*strength*glabella, 1.710]
    broad_targets = [1.551, 1.570, 1.599 + .008*strength*nose,
                     1.614 + .004*strength*nose,
                     old_targets[4], 1.710]
    if height <= 1.580 or height >= 1.608:
        if lateral == 1:
            return lateral, height
        targets = old_targets if height <= 1.580 else broad_targets
        mapped_q,mapped_y=mouth_shell_rounding(height,lateral,q,flow_monotone_curve(height, old_knots, targets),strength)
        return orbital_shell_rounding(height,lateral,mapped_q,mapped_y,strength)
    mouth_boundary = flow_monotone_curve(1.580, old_knots, old_targets)
    ear_boundary = flow_monotone_curve(1.608, old_knots, broad_targets)
    knots = [1.551, 1.570, 1.580, 1.599, 1.608, 1.614, 1.673, 1.710]
    targets = [1.551, 1.570, mouth_boundary,
               1.599 + strength*(.008*nose - .014*cheek),
               ear_boundary, broad_targets[3], old_targets[4], 1.710]
    yy = flow_monotone_curve(height, knots, targets)
    mapped_q,mapped_y=mouth_shell_rounding(height,lateral,q,yy,strength)
    return orbital_shell_rounding(height,lateral,mapped_q,mapped_y,strength)


def feature_flow_surface_point(height, lateral):
    """Evaluate the original continuous skin surface, including existing rim rules."""
    from math import exp
    q, y = feature_flow_chart(height, lateral)
    yy = (y - upper_boundary_offset(y, q)
          + H*C['chin_return_lift']*q*q*exp(-(y-1.544)/.008)
          + H*C['chin_front_lift']*exp(-(y-Y0)/(.02*H)))
    x = width(yy if y > Y0 + H*C['top_warp_start_height'] else y) * q
    z = upper_boundary_depth(x, yy, y, q)
    x, z = jaw_return_position(x, yy, z, q, depth(width(yy), yy))
    return coord(x, yy, z)


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    def replace(old, new):
        nonlocal shell
        if shell.count(old) != 1:
            raise ValueError('Feature-flow source contract changed: ' + old[:80])
        shell = shell.replace(old, new, 1)
    replace('    rows, cols = len(levels), len(lateral)\n    verts = []',
            '    rows, cols = len(levels), len(lateral)\n    verts = []\n    chart_reference = []\n    chart_samples = []')
    replace('            verts.append(coord(x, yy, zz))',
            '            chart_reference.append(coord(x, yy, zz))\n'
            '            mapped_q, mapped_y = feature_flow_chart(y, q)\n'
            '            chart_samples.append((j, i, y, q, mapped_y, mapped_q))\n'
            '            verts.append(feature_flow_surface_point(y, q))')
    replace('            x, _, y = verts[vi]', '            x, _, y = chart_reference[vi]')
    replace('boundary_mouth_angles([(verts[i][0],verts[i][2]) for i in boundary])',
            'boundary_mouth_angles([(chart_reference[i][0],chart_reference[i][2]) for i in boundary])')
    replace('        "semantic_rows": levels,',
            '        "feature_flow_layout": {"version": 2,"mouth_outer_frame_rounding":.12,"mouth_cheek_fade_width":.65, "chart_samples": chart_samples, "inner_aperture_reference": chart_reference, "scope": "Source chart sampling; native geometry and pose acceptance required"},\n'
            '        "semantic_rows": levels,')
    eye_ring_source = '    if kind == "eye":\n        return [0, 0.30, 0.65]'
    if out['face.py'].count(eye_ring_source) != 1:
        raise ValueError('Feature-flow eye ring source contract changed')
    out['face.py'] = out['face.py'].replace(
        eye_ring_source, '    if kind == "eye":\n        return [0, 0.6, 0.7, 0.9]', 1)
    out['shell.py'] = shell + '\n\n' + inspect.getsource(feature_flow_surface_point)
    helpers = (flow_smoothstep, flow_monotone_curve, orbital_shell_rounding, mouth_shell_rounding, feature_flow_chart)
    out['face.py'] += '\n\n' + '\n\n'.join(inspect.getsource(f) for f in helpers) + '\n'
    return out
