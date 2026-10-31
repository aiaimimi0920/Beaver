"""Graded source chin rows and a physically parameterized closure strip."""
import inspect


def feature_flow_rows(levels):
    if len(levels) != 22 or levels[:4] != [1.544, 1.551, 1.560, 1.570]:
        raise ValueError('Unexpected semantic chin row contract')
    return list(levels)


def feature_chin_row_columns(row, columns):
    """Storage slots are integer IDs; logical sampling columns are separate."""
    if columns != 13 or row not in (0, 1, 2, 3):
        raise ValueError('Unsupported graded chin row')
    return ([0,3,6,9,12], [0,2,4,6,8,10,12],
            [0,1,3,4,6,7,9,10,12], list(range(13)))[row]


def feature_chin_front_strip(vertices, row, levels, columns, sample):
    """Validated local source sampling; complete native gates remain required."""
    low = feature_chin_row_columns(row, columns)
    high = feature_chin_row_columns(row+1, columns)
    for r, slots in ((row,low),(row+1,high)):
        for i, slot in enumerate(slots):
            vertices[r*columns+slot] = sample(levels[r], 12*i/(len(slots)-1))
    starts = ((2,), (1,), (1,4))[row]
    params = (((.25,7.125),(.25,7.85),(.15,9.525)),
              ((.2,3),(.2,4.1),(.2,5.2)),
              ((.2,2.2),(.2,3),(.2,3.8),(.2,6.8),(.2,7.6),(.2,8.4)))[row]
    faces = []; k = j = pi = 0
    while k < len(low)-1:
        if k not in starts:
            faces.append((row*columns+low[k],row*columns+low[k+1],
                          (row+1)*columns+high[j+1],(row+1)*columns+high[j]))
            k += 1; j += 1
            continue
        a0,a1,a2 = [row*columns+low[k+i] for i in range(3)]
        b0,b1,b2,b3,b4 = [(row+1)*columns+high[j+i] for i in range(5)]
        left,middle,right = range(len(vertices),len(vertices)+3)
        for t,col in params[pi:pi+3]:
            vertices.append(sample(levels[row]+t*(levels[row+1]-levels[row]),col))
        bridge = (b1,b0,a0,left)
        if row == 2 and k == 4:
            # Redirect the lower mouth-frame strip through its authored
            # transition point, not through a final assembled vertex ID.
            previous = (row*columns+low[k-1], a0, b0,
                        (row+1)*columns+high[j-1])
            if not faces or faces[-1] != previous:
                raise ValueError('Lower mouth-frame source adjacency changed')
            aa, bb, cc, dd = previous
            faces[-1] = (left,b1,cc,dd)
            bridge = (left,dd,aa,bb)
        faces.extend(((a0,a1,middle,left),(middle,b2,b1,left),bridge,
                      (a1,a2,right,middle),(middle,right,b3,b2),(right,a2,b4,b3)))
        k += 2; j += 4; pi += 3
    if j != len(high)-1 or pi != len(params):
        raise ValueError('Incomplete graded source band')
    return faces


def flow_chain_knots(vertices, chain):
    from math import dist
    lengths = [0.0]
    for a, b in zip(chain, chain[1:]):
        lengths.append(lengths[-1] + dist(vertices[a], vertices[b]))
    if lengths[-1] <= 1e-9 or any(b-a <= 1e-10 for a,b in zip(lengths,lengths[1:])):
        raise ValueError('Degenerate source boundary chain')
    return [x/lengths[-1] for x in lengths]


def flow_chain_sample(vertices, chain, knots, value):
    value = max(0.0, min(1.0, value))
    i = next((i for i in range(len(knots)-1) if value <= knots[i+1]), len(knots)-2)
    t = (value-knots[i])/(knots[i+1]-knots[i])
    return tuple((1-t)*a+t*b for a,b in zip(vertices[chain[i]], vertices[chain[i+1]]))


def feature_chin_side_strip(vertices, levels, columns, side_path):
    """Choose front extent by physical height, not the underside array length."""
    from math import dist
    if len(side_path) != 5:
        raise ValueError('Graded underside must have one initial and four authored stations')
    end_height = vertices[side_path[-1]][2]
    candidates = [j for j in range(1, len(levels)) if levels[j] <= 1.590]
    end_row = min(candidates, key=lambda j: abs(vertices[j*columns+columns-1][2]-end_height))
    if end_row != 2:
        raise ValueError('Physical chin endpoint needs a different explicit quad-strip layout')
    front = [j*columns+columns-1 for j in range(3)]
    a0, a1, a2 = front
    b0, b1, b2, b3, b4 = side_path
    front_knots, back_knots = flow_chain_knots(vertices, front), flow_chain_knots(vertices, side_path)
    # Rotate the connectivity so the near-collinear b1-b0-a0 corner
    # spans two different quads. Sample a ruled surface, not vertex offsets.
    aa = [b2,b1,b0]
    bb = [b3,b4,a2,a1,a0]
    def ruled(u,t):
        pa = flow_chain_sample(vertices,aa,[0,.5,1],u)
        pb = flow_chain_sample(vertices,bb,[0,.25,.5,.75,1],u)
        return tuple((1-t)*x+t*y for x,y in zip(pa,pb))
    left,middle,right = range(len(vertices),len(vertices)+3)
    vertices.extend(ruled(u,t) for u,t in ((.35,.25),(.4,.25),(.65,.45)))
    x0,x1,x2 = aa
    y0,y1,y2,y3,y4 = bb
    faces = [(x0,x1,middle,left),(middle,y2,y1,left),(y1,y0,x0,left),
             (x1,x2,right,middle),(middle,right,y3,y2),(right,x2,y4,y3)]
    # Sidewall winding opposes the front-grid chart orientation.
    faces = [tuple(reversed(face)) for face in faces]
    return faces, {'front_rows': [0,1,end_row], 'front_arc_knots': front_knots,
                   'underside_arc_knots': back_knots, 'quads': len(faces),
                   'scope': 'Source connectivity and physical boundary correspondence; native gates still required'}


def compact_flow_vertices(vertices, faces, chains):
    used = sorted({index for face in faces for index in face})
    remap = {old:new for new,old in enumerate(used)}
    if any(index not in remap for chain in chains for index in chain):
        raise ValueError('A protected source boundary was removed')
    return ([vertices[i] for i in used], [tuple(remap[i] for i in face) for face in faces],
            [[remap[i] for i in chain] for chain in chains], remap)


def feature_flow_grid_point(height, column, lateral, nominal):
    from math import floor
    i = min(len(lateral)-2, int(floor(column)))
    t = column-i
    lat = lateral[i]*(1-t)+lateral[i+1]*t
    nom = nominal[i]*(1-t)+nominal[i+1]*t
    focus = max(0,min(1,(height-1.544)/.026))*max(0,min(1,(1.710-height)/.028))
    return feature_flow_surface_point(height, lat*focus+nom*(1-focus))


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    def replace(old, new):
        nonlocal shell
        if shell.count(old) != 1:
            raise ValueError('Chin-flow source contract changed: ' + old[:80])
        shell = shell.replace(old, new, 1)
    replace('    levels = density_rows(levels)', '    levels = feature_flow_rows(levels)')
    replace('"chart_samples": chart_samples',
            '"initial_dense_chart_samples": chart_samples, "sample_metadata_scope": "Initial chart before sparse chin resampling; not final mesh vertex IDs"')
    replace('            shell_faces.extend(chin_strip_faces(j, cols))',
            '            shell_faces.extend(feature_chin_front_strip(verts, j, levels, cols, lambda y, col: feature_flow_grid_point(y, col, lateral, nominal)))')
    replace('    chin_columns = chin_column_indices(cols)', '    chin_columns = feature_chin_row_columns(0, cols)')
    replace('    first_back,first_lift,first_spread = C["chin_underside_stations"][0]\n'
            '    support_stations = [(first_back*t,first_lift*t,1+(first_spread-1)*t) for t in (1/3,2/3)]\n'
            '    for back, lift, spread in support_stations + C["chin_underside_stations"]:',
            '    for back, lift, spread in C["chin_underside_stations"]:')
    replace('    outer = [j * cols + cols - 1 for j in range(len(side_path))]\n'
            '    shell_faces.append((outer[0], side_path[0], side_path[1], outer[1]))\n'
            '    for j in range(1, len(side_path) - 1):\n'
            '        shell_faces.append((side_path[j], side_path[j + 1], outer[j + 1], outer[j]))',
            '    side_flow_faces, side_flow_report = feature_chin_side_strip(verts, levels, cols, side_path)\n'
            '    shell_faces.extend(side_flow_faces)')
    replace('range(len(side_path)-1, rows)', 'range(side_flow_report["front_rows"][-1], rows)')
    replace('    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)',
            '    flow_chains = [chin_columns, rear_ids, outline_trace["vertex_ids"]]\n'
            '    if STYLE.get("sampling_outline_refinement",0): flow_chains.append(chain)\n'
            '    verts, shell_faces, flow_chains, flow_remap = compact_flow_vertices(verts, shell_faces, flow_chains)\n'
            '    chin_columns, rear_ids, outline_trace["vertex_ids"] = flow_chains[:3]\n'
            '    if STYLE.get("sampling_outline_refinement",0): chain = flow_chains[3]\n'
            '    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)')
    replace('        "chin_side_quads": len(side_path) - 1,',
            '        "chin_flow_layout": {"version": 2, "lower_mouth_strip_rotation": True, "front_row_columns": [len(feature_chin_row_columns(j,cols)) for j in range(4)], "closure_rows": len(under_depth_rows), "side": side_flow_report},\n'
            '        "chin_side_quads": len(side_flow_faces),')
    out['shell.py'] = shell + '\n\n' + inspect.getsource(feature_flow_grid_point)
    helpers = (feature_flow_rows, feature_chin_row_columns, feature_chin_front_strip,
               flow_chain_knots, flow_chain_sample, feature_chin_side_strip, compact_flow_vertices)
    out['face.py'] += '\n\n' + '\n\n'.join(inspect.getsource(f) for f in helpers) + '\n'
    return out
