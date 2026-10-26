"""Coherent lip relief correction with an unchanged inherited-depth reference."""
import inspect


def source_lip_relief_delta(x, y):
    """Reduce the coupled lip peaks and seam, not the shoulder or pigment."""
    original = lip_detail_depth(x,y)
    shoulder = lip_shoulder_support(x,y-mouth_profile_shift(x))
    return (.45-1.0)*(original-shoulder)


def adapt(sources):
    out = dict(sources)
    old = '    return inherited_base_depth(x, y) + base_delta(x, y, C, curve, DATA)'
    if out['face.py'].count(old) != 1:
        raise ValueError('Lip relief correction source contract changed')
    out['face.py'] = out['face.py'].replace(old,old+' + source_lip_relief_delta(x,y)',1)
    out['face.py'] += '\n\n'+inspect.getsource(source_lip_relief_delta)+'\n'
    marker = '        "semantic_rows": levels,'
    if out['shell.py'].count(marker) != 1:
        raise ValueError('Lip relief report contract changed')
    out['shell.py'] = out['shell.py'].replace(marker,
        '        "coherent_lip_source_relief": {"version":1,"relief_and_seam_scale":.45,"shoulder_unchanged":True,"shape_changed":True,"reference":"original C and inherited_base_depth unchanged","displacement_gate_unchanged":True,"station_policy":"natural pose-curvature resampling; no fixed vertex patch"},\n'+marker)
    return out
