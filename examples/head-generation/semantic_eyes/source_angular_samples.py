"""Opt-in exact shoulder stations for eyelids, pocket and primary ink."""


import ast
from sampling_contract import lid_station_plan


def adapt_angular_samples(sources, enabled, sampling=None):
    count = (sampling or {}).get("sampling_lid_stations", 0)
    if not enabled and not count:
        return sources
    result = dict(sources)
    face = result["face.py"]
    old = "        cols = 33"
    new = "        stations=" + repr(list(lid_station_plan(count, enabled))) + "\n        cols=len(stations)"
    if face.count(old) != 1:
        raise ValueError("Angular lid station contract changed")
    face = face.replace(old,new,1)
    old = "                    t = -1 + 2 * i / (cols - 1)"
    if face.count(old) != 2:
        raise ValueError("Angular lid sample contract changed")
    face = face.replace(old,"                    t = stations[i]")
    result["face.py"] = face
    if not enabled:
        return result
    anchor = "    h = xs[i + 1] - xs[i]"
    curves = [node for node in ast.parse(face).body
              if isinstance(node, ast.FunctionDef) and node.name == "curve"]
    if len(curves) != 1:
        raise ValueError("Expected one primary ink curve function")
    lines = face.splitlines(keepends=True)
    first, last = curves[0].lineno - 1, curves[0].end_lineno
    primary = "".join(lines[first:last])
    if primary.count(anchor) != 1:
        raise ValueError("Angular ink interpolation contract changed")
    primary = primary.replace(anchor,anchor+"\n"
        '    if points is globals().get("STYLE",{}).get("upper_lash_rails"):\n'
        '        if abs(abs(xs[i])-.55)<1e-10: ds[i]=slopes[i]\n'
        '        if abs(abs(xs[i+1])-.55)<1e-10: ds[i+1]=slopes[i]',1)
    face = "".join(lines[:first]) + primary + "".join(lines[last:])
    result["face.py"] = face
    socket = result["eye_socket.py"]
    old = "    contours = [eye_contour(side, cos(2 * pi * k / n), sin(2 * pi * k / n) >= 0) for k in range(n)]"
    new = '''    angles=[2*pi*k/n for k in range(n)]
    for shoulder in (-.55,.55):
        angle=math.acos(shoulder)
        angles[round(angle*n/(2*pi))]=angle
    assert all(a<b for a,b in zip(angles,angles[1:]))
    contours=[eye_contour(side,cos(angle),sin(angle)>=0) for angle in angles]'''
    if socket.count(old) != 1:
        raise ValueError("Angular pocket sample contract changed")
    socket = socket.replace(old,new,1)
    old_angle = "            angle = 2 * pi * k / n"
    if socket.count(old_angle) != 1:
        raise ValueError("Angular pocket ring contract changed")
    result["eye_socket.py"] = socket.replace(old_angle,"            angle = angles[k]",1)
    return result
