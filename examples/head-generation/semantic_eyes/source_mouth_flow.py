"""Explicit mouth source-shape and curvature sampling candidate.

This is a shape change, not merely reindexing the old elliptical aperture.
Native aperture, morph, welding and visual acceptance remain mandatory.
"""
import ast
import inspect


def mouth_half_contour(a):
    from math import cos, sin, sqrt, copysign, pi
    co = 0.0 if abs(a) >= pi/2-1e-12 else max(0.0, cos(a))
    si = sin(a)
    x = H*C['mouth_half_width']*sqrt(co)
    return x, rest_seam_y(x)+REST_GAP*copysign(sqrt(abs(si)),si)


def mouth_flow_point(a):
    from math import sin
    x,y = mouth_half_contour(float(a))
    key = 'upper_lip_edge_depth_delta' if a >= 0 else 'lower_lip_edge_depth_delta'
    dz = H*C[key]*sin(a)**2
    return coord(x,y,depth(x,y)+dz)


def mouth_turn_stations(lo, hi, count):
    import numpy as np
    if count < 2:
        raise ValueError('Mouth arc must include both semantic endpoints')
    u = np.linspace(0,1,8001)
    t = lo+(hi-lo)*(.5-.5*np.cos(np.pi*u))
    p = np.array([mouth_flow_point(a) for a in t])
    delta = np.array([tuple(jaw_delta(x,height,-negative_depth)) for x,negative_depth,height in p])
    pose_turns = []
    for pose in (p,p+delta):
        d = np.diff(pose,axis=0)
        lengths = np.linalg.norm(d,axis=1)
        if not np.isfinite(pose).all() or np.any(lengths <= 0):
            raise ValueError('Degenerate mouth source curve samples')
        d /= lengths[:,None]
        pose_turns.append(np.arccos(np.clip((d[:-1]*d[1:]).sum(1),-1,1)))
    accumulated = np.r_[0,np.cumsum(pose_turns[0]+pose_turns[1])]
    if accumulated[-1] <= 1e-8:
        raise ValueError('Mouth source curvature cannot allocate stations')
    mid = (t[:-1]+t[1:])*.5
    result = np.interp(np.linspace(0,accumulated[-1],count),accumulated,mid)
    result[0],result[-1] = lo,hi
    if np.any(np.diff(result) <= 0):
        raise ValueError('Mouth curvature stations reverse or duplicate')
    return [float(a) for a in result]


def boundary_mouth_angles(points):
    from math import atan2, pi
    half_width = max(abs(x) for x,y in points)
    span = max(y for x,y in points)-min(y for x,y in points)
    if half_width <= 0 or span <= 0:
        raise ValueError('Degenerate mouth source boundary')
    rays = [atan2((y-MOUTH_Y)/(span/2),x/half_width) for x,y in points]
    corner = min(range(len(rays)),key=lambda i:abs(rays[i]))
    if not 1 < corner < len(points)-2:
        raise ValueError('Mouth boundary lacks two-sided corner support')
    lower = mouth_turn_stations(-pi/2,0,corner+1)
    upper = mouth_turn_stations(0,pi/2,len(points)-corner)
    return lower+upper[1:]


def align_mouth_source_corner(vertices, levels, lateral, columns):
    """Align a semantic source row, before topology or per-vertex attributes."""
    row = levels.index(1.580)
    column = lateral.index(.44)
    q,_ = feature_flow_chart(1.580,.44)
    xcorner = H*C['mouth_half_width']
    # Shared facial-surface evaluator includes the active mouth profile.
    y = mouth_rest_seam_y(xcorner)
    x = width(y)*q
    z = depth(x,y)
    x,z = jaw_return_position(x,y,z,q,depth(width(y),y))
    index = row*columns+column
    before = tuple(vertices[index])
    vertices[index] = coord(x,y,z)
    return {'source_height':1.580,'source_lateral':.44,
            'before':before,'after':tuple(vertices[index]),
            'corner_half_width_m':xcorner,
            'scope':'Source corner alignment; topology and native acceptance required'}


def replace_function(source, name, replacement):
    matches = [node for node in ast.parse(source).body
               if isinstance(node,ast.FunctionDef) and node.name == name]
    if len(matches) != 1:
        raise ValueError('Mouth source function contract changed: '+name)
    node = matches[0]
    lines = source.splitlines(keepends=True)
    return ''.join(lines[:node.lineno-1])+replacement+'\n'+''.join(lines[node.end_lineno:])


def adapt(sources):
    out = dict(sources)
    mouth = out['mouth.py']
    for helper in (mouth_half_contour,boundary_mouth_angles):
        mouth = replace_function(mouth,helper.__name__,inspect.getsource(helper))
    out['mouth.py'] = mouth+'\n\n'+inspect.getsource(mouth_flow_point)+'\n\n'+inspect.getsource(mouth_turn_stations)
    shell = out['shell.py']
    old = 'dz = H * C[rim_key] * abs(sin(a)) * (1 - blend) ** 2'
    if shell.count(old) != 1:
        raise ValueError('Mouth lip depth source contract changed')
    shell = shell.replace(old,'dz = H * C[rim_key] * sin(a)**2 * (1 - blend) ** 2',1)
    marker = '    shell_faces = []'
    if shell.count(marker) != 1:
        raise ValueError('Mouth source grid ownership changed')
    shell = shell.replace(marker,'    mouth_flow_report = align_mouth_source_corner(verts,levels,lateral,cols)\n'+marker,1)
    marker = '        "semantic_rows": levels,'
    if shell.count(marker) != 1:
        raise ValueError('Mouth source report contract changed')
    shell = shell.replace(marker,'        "mouth_feature_flow": {"version":1,"contour":"superellipse4","lip_depth":"sin_squared","stations":"neutral_open_cumulative_3d_turn_sum","corner":mouth_flow_report,"shape_changed":True},\n'+marker,1)
    out['shell.py'] = shell+'\n\n'+inspect.getsource(align_mouth_source_corner)
    return out
