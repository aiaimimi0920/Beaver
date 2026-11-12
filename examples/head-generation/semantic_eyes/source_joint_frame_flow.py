"""Joint outer mouth-frame sampling after protected lip construction."""
import inspect


def joint_frame_chart(height, lateral, strength=1.0):
    """Compose a separate chart; never replace the protected mouth chart."""
    mq, my = feature_flow_chart(height,lateral,strength)
    nx, ny = lateral/.44, (height-1.5845)/.0145
    window = (flow_smoothstep((nx-.5)/.5)
              * (1-flow_smoothstep((nx-1)/.65))
              * (1-flow_smoothstep((abs(ny)-1)/(.5 if ny>0 else .25))))
    extra = .02*flow_smoothstep(ny)*strength*window
    mq -= extra*.44*nx*ny*ny
    my -= extra*.0145*ny*nx*nx
    if not 1.599 < height < 1.614 or not .18 < lateral < .78:
        return mq,my
    nodes = (.18,.25,.34,.44,.55,.67,.78)
    heights = (1.599,1.6045,1.608,1.614)
    zero = (0.,)*len(nodes)
    dq = (zero,
          (0.,-.0009798696397038292,-.0022348120530970112,-.02,-.02,-.0049935501240333195,0.),
          (0.,-.0010178551711066677,-.0017309866238179883,-.014020424655750357,-.014019579747055026,-.0035763655069871577,0.),zero)
    dy = (zero,
          (0.,-.0005782223082876906,-.0013184361524736805,-.0016576393903896583,-.0016741847001290713,-.0009503679391827322,0.),
          (0.,-.00020675166340674338,-.0005535134487516816,-.000638471451303273,-.0007122506462885823,-.0008852398333342753,0.),zero)
    k = next(i for i in range(6) if nodes[i] <= lateral <= nodes[i+1])
    j = next(i for i in range(3) if heights[i] <= height <= heights[i+1])
    t = flow_smoothstep((lateral-nodes[k])/(nodes[k+1]-nodes[k]))
    u = flow_smoothstep((height-heights[j])/(heights[j+1]-heights[j]))
    def blend(values):
        low = values[j][k]*(1-t)+values[j][k+1]*t
        high = values[j+1][k]*(1-t)+values[j+1][k+1]*t
        return strength*(low*(1-u)+high*u)
    return mq+blend(dq),my+blend(dy)


def joint_frame_surface_point(height, lateral):
    """Use the original continuous surface and all existing depth guards."""
    from math import exp
    q, y = joint_frame_chart(height,lateral)
    yy = (y-upper_boundary_offset(y,q)
          + H*C['chin_return_lift']*q*q*exp(-(y-1.544)/.008)
          + H*C['chin_front_lift']*exp(-(y-Y0)/(.02*H)))
    x = width(yy if y > Y0+H*C['top_warp_start_height'] else y)*q
    z = upper_boundary_depth(x,yy,y,q)
    x,z = jaw_return_position(x,yy,z,q,depth(width(yy),yy))
    return coord(x,yy,z)


def resample_joint_frame(vertices, levels, lateral, columns, sample):
    """Address initial semantic grid roles before compaction or mesh creation."""
    from math import dist, isfinite
    if columns != 13 or len(lateral) != columns or len(levels) != 22:
        raise ValueError('Joint-frame source grid contract changed')
    stations = [(1.590,q) for q in (.44,.55,.67)]
    stations += [(h,q) for h in (1.599,1.6045,1.608) for q in (.25,.34,.44,.55,.67)]
    changes = []
    for h,q in stations:
        if levels.count(h) != 1 or lateral.count(q) != 1:
            raise ValueError('Joint-frame semantic station missing or duplicated')
        row,col = levels.index(h),lateral.index(q)
        if row <= 3:
            raise ValueError('Joint-frame cannot address sparse chin storage slots')
        index = row*columns+col
        before,after = tuple(vertices[index]),tuple(sample(h,q))
        move = dist(before,after)
        if not all(isfinite(v) for v in after) or move > .003:
            raise ValueError('Joint-frame source sample exceeded reviewed movement')
        changes.append((index,after,h,q,move))
    for index,after,_,_,_ in changes:
        vertices[index] = after
    return {'stations':[[h,q] for _,_,h,q,_ in changes],
            'max_displacement_m':max(move for *_,move in changes),
            'protected_mouth_chart':'unchanged .12 frame through all internal solvers',
            'internal_solver_evidence_scope':'pre_outer_resample',
            'scope':'Outer source grid only; full native visual and pose validation required'}


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    marker = '    flow_chains = [chin_columns, rear_ids, outline_trace["vertex_ids"]]'
    if shell.count(marker) != 1:
        raise ValueError('Joint-frame must run before source compaction')
    shell = shell.replace(marker,
        '    joint_frame_report = resample_joint_frame(verts,levels,lateral,cols,joint_frame_surface_point)\n'+marker,1)
    marker = '        "semantic_rows": levels,'
    if shell.count(marker) != 1:
        raise ValueError('Joint-frame report contract changed')
    shell = shell.replace(marker,'        "joint_outer_frame_flow": joint_frame_report,\n'+marker,1)
    boundary = 'def build_face('
    if shell.count(boundary) != 1:
        raise ValueError('Joint-frame geometry responsibility boundary changed')
    helpers = inspect.getsource(joint_frame_surface_point)+'\n\n'+inspect.getsource(resample_joint_frame)+'\n\n'
    out['shell.py'] = shell.replace(boundary,helpers+boundary,1)
    out['face.py'] += '\n\n'+inspect.getsource(joint_frame_chart)+'\n'
    return out
