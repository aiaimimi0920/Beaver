"""Distribute orbital rings without moving the aperture or shared outer frame."""
import inspect


def eye_source_ring_fraction(base, angle):
    for old,new in ((.6,.25),(.7,.5),(.9,.75)):
        if abs(base-old)<1e-12: return new
    return base


def adapt(sources):
    out = dict(sources); shell = out['shell.py']
    def replace(old,new):
        nonlocal shell
        if shell.count(old) != 1:
            raise ValueError('Eye-ring source contract changed: '+old[:70])
        shell = shell.replace(old,new,1)
    replace('        inner = []','        inner = []\n        eye_source_angles = []')
    replace('            inner.append((xx, yy))',
            '            inner.append((xx, yy))\n            eye_source_angles.append(a)')
    replace('                x = ix * (1 - blend) + bx * blend\n                y = iy * (1 - blend) + by * blend',
            '                sample_blend = eye_source_ring_fraction(blend,eye_source_angles[k]) if kind == "eye" else blend\n'
            '                x = ix * (1 - sample_blend) + bx * sample_blend\n'
            '                y = iy * (1 - sample_blend) + by * sample_blend')
    replace('        "semantic_rows": levels,',
        '        "eye_curvature_source_sampling": {"version":2,"base_rings":[0,.6,.7,.9,1],"effective_rings":[0,.25,.5,.75,1],"aperture_and_frame_fixed":True,"surface_changed":False},\n'
            '        "semantic_rows": levels,')
    out['shell.py'] = shell
    out['face.py'] += '\n\n'+inspect.getsource(eye_source_ring_fraction)+'\n'
    return out
