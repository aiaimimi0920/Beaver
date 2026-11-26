"""C1 orbital support outside the aperture, retaining the original bound reference."""
import inspect


def orbital_endpoint_weight(t, dy, upper, calibration, height):
    from math import sqrt, exp
    c = calibration
    arch = c['eye_upper_arch'] if upper else c['eye_lower_arch']
    power = c['eye_upper_power'] if upper else c['eye_lower_power']
    horizontal = c['eye_horizontal_power'] if upper else c['eye_lower_horizontal_power']
    q = eye_warp(t,c) if upper else lower_coordinate(t,c)
    u = abs(dy)/(height*arch)
    r = sqrt(abs(q)**horizontal+u**(1/power))
    lateral = max(0,min(1,(1+c['orbital_lateral_fade']-abs(t))/c['orbital_lateral_fade']))
    lateral = lateral*lateral*(3-2*lateral)
    band = c['orbital_band_width']
    weight = exp(-((r-1)/band)**2)*lateral
    dr = (1 if dy>0 else -1)*u**(1/power-1)/(2*r*power*height*arch)
    return weight, weight*(-2*(r-1)/band**2)*dr


def orbital_continuous_weight(t, dy, original_weight, calibration, height):
    """Bridge only outside the true eye, matching weight and slope at both ends."""
    fade = calibration['orbital_lateral_fade']
    h = .002*max(0,min(1,(abs(t)-1)/fade))
    if h <= 0 or abs(dy) >= h:
        return original_weight
    low,dlow = orbital_endpoint_weight(t,-h,False,calibration,height)
    high,dhigh = orbital_endpoint_weight(t,h,True,calibration,height)
    s = (dy+h)/(2*h)
    value = ((2*s**3-3*s*s+1)*low+(s**3-2*s*s+s)*2*h*dlow
             +(-2*s**3+3*s*s)*high+(s**3-s*s)*2*h*dhigh)
    if not 0 <= value <= 1:
        raise ValueError('Orbital C1 weight exceeded its physical interval')
    return value


def adapt(sources):
    out = dict(sources)
    old = '    result = raw + (rim_depth(x, y, False) - raw) * weight'
    if out['face.py'].count(old) != 1:
        raise ValueError('Orbital depth source contract changed')
    new = ('    continuous_weight = orbital_continuous_weight(t,y-cy,weight,C,H)\n'
           '    result = raw + (rim_depth(x, y, False) - raw) * continuous_weight')
    out['face.py'] = out['face.py'].replace(old,new,1)
    # Keep the following inherited-reference expression on the ORIGINAL weight.
    # The reviewed displacement threshold is unchanged; no clamping hides failure.
    out['face.py'] += '\n\n'+'\n\n'.join(inspect.getsource(f) for f in
        (orbital_endpoint_weight,orbital_continuous_weight))+'\n'
    marker = '        "semantic_rows": levels,'
    if out['shell.py'].count(marker) != 1:
        raise ValueError('Orbital continuity report contract changed')
    out['shell.py'] = out['shell.py'].replace(marker,
        '        "orbital_source_continuity": {"version":1,"outside_aperture_only":True,"maximum_vertical_bridge_m":.002,"fixed_reference":"original orbital weight and existing reviewed bound","visual_acceptance":False},\n'+marker)
    return out
