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
    if abs(base-.75)<1e-12: return base-(.75-.55)*upper
    return base


def adapt(sources):
    out = dict(sources)
    old = '    return inherited_base_depth(x, y) + base_delta(x, y, C, curve, DATA)'
    if out['face.py'].count(old) != 1:
        raise ValueError('Lip relief correction source contract changed')
    out['face.py'] = out['face.py'].replace(old,old+' + source_lip_relief_delta(x,y)',1)
    out['face.py'] += '\n\n'+inspect.getsource(source_lip_relief_delta)+'\n'+inspect.getsource(lip_source_ring_fraction)+'\n'
    old_ring = 'eye_source_ring_fraction(blend,eye_source_angles[k]) if kind == "eye" else blend'
    old_depth = '(1 - blend) ** 2'
    if out['shell.py'].count(old_ring)!=1 or out['shell.py'].count(old_depth)!=1:
        raise ValueError('Lip radial source sampling contract changed')
    out['shell.py'] = out['shell.py'].replace(old_ring,
        'eye_source_ring_fraction(blend,eye_source_angles[k]) if kind == "eye" else lip_source_ring_fraction(blend,mouth_angles[k])',1)
    out['shell.py'] = out['shell.py'].replace(old_depth,'(1 - sample_blend) ** 2',1)
    marker = '        "semantic_rows": levels,'
    if out['shell.py'].count(marker) != 1:
        raise ValueError('Lip relief report contract changed')
    out['shell.py'] = out['shell.py'].replace(marker,
        '        "coherent_lip_source_relief": {"version":2,"relief_and_seam_scale":.45,"shoulder_unchanged":True,"shape_changed":True,"reference":"original C and inherited_base_depth unchanged","displacement_gate_unchanged":True,"station_policy":"upper radial rings .5 to .175 and .75 to .55 with sin(angle)^2 fade; lower lip and boundaries fixed"},\n'+marker)
    return out
