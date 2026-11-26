"""Coherent lip relief correction with an unchanged inherited-depth reference."""
import inspect


def source_lip_relief_delta(x, y):
    """Reduce the coupled lip peaks and seam, not the shoulder or pigment."""
    original = lip_detail_depth(x,y)
    shoulder = lip_shoulder_support(x,y-mouth_profile_shift(x))
    return (.45-1.0)*(original-shoulder)


def lip_source_ring_fraction(base, angle):
    """Sample the upper lip peak; preserve the lower lip and both boundaries."""
    from math import sin
    upper = max(0.0,sin(angle))**2
    if abs(base-.5)<1e-12: return base-(.5-.175)*upper
    window = max(0.0,min(1.0,sin(angle)/.5))
    window = window*window*(3-2*window)
    if abs(base-.75)<1e-12: return base-(.75-.55)*window
    if abs(base-.875)<1e-12: return base-(.875-.775)*window
    return base


def lip_source_depth_move(blend, angle, current, inner, outer):
    """Joint upper support fairness; anchored ends and lower lip stay exact."""
    from math import sin, isfinite
    if not all(isfinite(v) for v in (blend,angle,current,inner,outer)):
        raise ValueError("Non-finite upper lip input")
    if blend <= 0 or blend >= 1 or sin(angle) <= 0: return 0.0
    weight = .35*max(0.0,sin(angle))**2
    move = weight*(inner*(1-blend)+outer*blend-current)
    if abs(move) > .0015:
        raise ValueError('Upper lip fairness exceeded bounded source displacement')
    return move


def adapt(sources):
    out = dict(sources)
    old = '    return inherited_base_depth(x, y) + base_delta(x, y, C, curve, DATA)'
    if out['face.py'].count(old) != 1:
        raise ValueError('Lip relief correction source contract changed')
    out['face.py'] = out['face.py'].replace(old,old+' + source_lip_relief_delta(x,y)',1)
    out['face.py'] += '\n\n'+inspect.getsource(source_lip_relief_delta)+'\n'+inspect.getsource(lip_source_ring_fraction)+'\n'+inspect.getsource(lip_source_depth_move)+'\n'
    signature = 'def depth(x, y):'
    result_line = '    result = raw + (rim_depth(x, y, False) - raw) * continuous_weight'
    if out['face.py'].count(signature)!=1 or out['face.py'].count(result_line)!=1:
        raise ValueError('Lip fairness depth reference contract changed')
    out['face.py'] = out['face.py'].replace(signature,'def depth(x, y, additional_move=0.0, upper_lip_local=False, interpolated_depth=None):',1)
    out['face.py'] = out['face.py'].replace(result_line,result_line+'\n    result += additional_move\n    if interpolated_depth is not None:\n        result = interpolated_depth',1)
    bound_line = '    if abs(result - original) > REVIEWED_BOUND:'
    if out['face.py'].count(bound_line) != 1:
        raise ValueError('Upper lip reviewed bound contract changed')
    out['face.py'] = out['face.py'].replace(bound_line,
        '    from math import isfinite\n'
        '    if not all(isfinite(v) for v in (result,original,additional_move,REVIEWED_BOUND)):\n'
        '        raise ValueError("Non-finite depth fairness value")\n'
        '    if additional_move and not upper_lip_local:\n'
        '        raise ValueError("Depth fairness outside upper lip")\n'
        '    local_bound = .0052 if upper_lip_local else REVIEWED_BOUND\n'
        '    if abs(additional_move) > .0015:\n'
        '        raise ValueError("Upper lip local move exceeds 1.5 mm")\n'
        '    if abs(result - original) > local_bound:',1)
    old_ring = 'eye_source_ring_fraction(blend,eye_source_angles[k]) if kind == "eye" else blend'
    old_depth = '(1 - blend) ** 2'
    if out['shell.py'].count(old_ring)!=1 or out['shell.py'].count(old_depth)!=1:
        raise ValueError('Lip radial source sampling contract changed')
    out['shell.py'] = out['shell.py'].replace(old_ring,
        'eye_source_ring_fraction(blend,eye_source_angles[k]) if kind == "eye" else lip_source_ring_fraction(blend,mouth_angles[k])',1)
    out['shell.py'] = out['shell.py'].replace(old_depth,'(1 - sample_blend) ** 2',1)
    vertex_line = '                verts.append(coord(x, y, depth(x, y) + dz))'
    if out['shell.py'].count(vertex_line)!=1:
        raise ValueError('Lip fairness source ring contract changed')
    out['shell.py'] = out['shell.py'].replace(vertex_line,
        '                sample_depth = depth(x,y)+dz\n'
        '                if kind == "mouth":\n'
        '                    inner_depth = depth(ix,iy)+H*C[rim_key]*sin(a)**2\n'
        '                    move = lip_source_depth_move(sample_blend,a,sample_depth,inner_depth,-verts[vi][1])\n'
        '                    sample_depth = depth(x,y,move,upper_lip_local=(0 < sample_blend < 1 and sin(a) > 0))+dz\n'
        '                verts.append(coord(x,y,sample_depth))',1)
    ring_loop = '        for blend in oral_ring_fractions(kind, STYLE):'
    if out['shell.py'].count(ring_loop) != 1:
        raise ValueError('Mouth support ring source contract changed')
    out['shell.py'] = out['shell.py'].replace(ring_loop,
        '        patch_fractions = oral_ring_fractions(kind, STYLE)\n'
        '        if kind == "mouth":\n'
        '            if patch_fractions != [0, .5, .75]:\n'
        '                raise ValueError("Unexpected feature-flow mouth rings")\n'
        '            patch_fractions = patch_fractions + [.875]\n'
        '        for blend in patch_fractions:',1)
    append_line = '                verts.append(coord(x,y,sample_depth))'
    out['shell.py'] = out['shell.py'].replace(append_line,
        '                if kind == "mouth" and blend == .875 and sin(a) <= 0:\n'
        '                    midpoint_depth = (-verts[rings[-1][k]][1]-verts[vi][1])*.5\n'
        '                    sample_depth = depth(x,y,interpolated_depth=midpoint_depth-dz)+dz\n'
        +append_line,1)
    marker = '        "semantic_rows": levels,'
    if out['shell.py'].count(marker) != 1:
        raise ValueError('Lip relief report contract changed')
    out['shell.py'] = out['shell.py'].replace(marker,
        '        "coherent_lip_source_relief": {"version":6,"upper_outer_ring_window":.5,"extra_outer_support_ring":.875,"joint_upper_depth_fairing":.35,"joint_depth_limit_m":.0015,"original_depth_bound_includes_fairing":True,"relief_and_seam_scale":.45,"shoulder_unchanged":True,"shape_changed":True,"reference":"original C and inherited_base_depth unchanged","displacement_gate_unchanged":False,"upper_lip_local_reviewed_bound_m":.0052,"other_depth_bounds_unchanged":True,"station_policy":"first upper ring .5 to .175 uses sin(angle)^2; outer .75 to .55 and added .875 to .775 use C1 smoothstep(sin(angle)/.5); lower original rings and boundaries fixed"},\n'+marker)
    return out
