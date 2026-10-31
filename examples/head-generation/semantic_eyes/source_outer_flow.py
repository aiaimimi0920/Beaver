"""Arc-length source stations with conforming quad transitions at both ends."""
import inspect


def outer_flow_q(height, column, lateral, nominal):
    i = min(len(lateral)-2, int(column))
    t = column-i
    focus = max(0,min(1,(height-1.544)/.026))*max(0,min(1,(1.710-height)/.028))
    return ((1-t)*lateral[i]+t*lateral[i+1])*focus + ((1-t)*nominal[i]+t*nominal[i+1])*(1-focus)


def outer_flow_interp(values, u):
    v = max(0,min(1,u))*(len(values)-1)
    i = min(len(values)-2,int(v))
    return values[i]*(1-(v-i))+values[i+1]*(v-i)


def outer_flow_arc_stations(height, q0, sample):
    from math import dist
    from bisect import bisect_left
    qs = [q0+(1-q0)*i/4000 for i in range(4001)]
    points = [sample(height,q) for q in qs]
    lengths = [0.0]
    for a,b in zip(points,points[1:]): lengths.append(lengths[-1]+dist(a,b))
    if lengths[-1] <= 1e-8: raise ValueError('Degenerate outer source curve')
    stations = [q0]
    for fraction in (1/3,2/3):
        target = fraction*lengths[-1]
        j = bisect_left(lengths,target)
        t = (target-lengths[j-1])/(lengths[j]-lengths[j-1])
        stations.append(qs[j-1]*(1-t)+qs[j]*t)
    return stations+[1.0], lengths[-1]


def outer_flow_fan(vertices, a, b, aq, bq, ya, yb, t, us, sample, reverse=False):
    left,middle,right = range(len(vertices),len(vertices)+3)
    parameters = []
    for u in us:
        q = (1-t)*outer_flow_interp(aq,u)+t*outer_flow_interp(bq,u)
        y = (1-t)*ya+t*yb
        parameters.append([y,q]); vertices.append(sample(y,q))
    a0,a1,a2 = a; b0,b1,b2,b3,b4 = b
    faces = [(a0,a1,middle,left),(middle,b2,b1,left),(b1,b0,a0,left),
             (a1,a2,right,middle),(middle,right,b3,b2),(right,a2,b4,b3)]
    if reverse: faces = [tuple(reversed(f)) for f in faces]
    return faces, {'vertex_ids':[left,middle,right], 'parameters':parameters}


def build_outer_source_flow(vertices, faces, levels, cols, lateral, nominal, sample):
    if cols != 13 or len(levels) != 22 or levels[3] != 1.570 or levels[-2:] != [1.760,1.780]:
        raise ValueError('Outer flow requires the versioned 22 by 13 source chart')
    original_count = len(faces); replacements = {}; rows = {}; report = []
    def q(y,col): return outer_flow_q(y,col,lateral,nominal)
    def replace(old,new):
        key = frozenset(old)
        if key in replacements: raise ValueError('Duplicate outer source replacement')
        replacements[key] = new
    for j in range(3,21):
        y = levels[j]; qs,length = outer_flow_arc_stations(y,q(y,11),sample)
        ids = [j*cols+11,len(vertices),len(vertices)+1,j*cols+12]
        vertices.extend(sample(y,value) for value in qs[1:3])
        rows[j] = (ids,qs)
        report.append({'height':y,'vertex_ids':ids,'q':qs,'arc_length_m':length})
    for j in range(3,20):
        a,_ = rows[j]; b,_ = rows[j+1]
        replace((j*cols+11,j*cols+12,(j+1)*cols+12,(j+1)*cols+11),
                [(a[k],a[k+1],b[k+1],b[k]) for k in range(3)])
    a = [2*cols+i for i in (9,10,12)]
    b = [3*cols+10]+rows[3][0]
    low,low_report = outer_flow_fan(vertices,a,b,[q(levels[2],c) for c in (9,10.5,12)],
        [q(levels[3],10)]+rows[3][1],levels[2],levels[3],.35,[.45,.5,.8],sample)
    replace((a[0],a[1],3*cols+11,3*cols+10),low)
    replace((a[1],a[2],3*cols+12,3*cols+11),[])
    a = [21*cols+i for i in (10,11,12)]
    b = [20*cols+10]+rows[20][0]
    high,high_report = outer_flow_fan(vertices,a,b,[q(levels[21],c) for c in (10,11,12)],
        [q(levels[20],10)]+rows[20][1],levels[21],levels[20],.7,[.3,.5,.8],sample,True)
    replace((20*cols+10,20*cols+11,21*cols+11,21*cols+10),high)
    replace((20*cols+11,20*cols+12,21*cols+12,21*cols+11),[])
    result = []; seen = set()
    for face in faces:
        key = frozenset(face)
        if key in replacements:
            if key in seen: raise ValueError('Duplicate original outer face')
            seen.add(key); result.extend(replacements[key])
        else: result.append(face)
    if seen != set(replacements) or len(result)-original_count != 42:
        raise ValueError('Incomplete outer source connectivity replacement')
    return result, {'version':1,'rows':report,'lower_transition':low_report,
        'upper_transition':high_report,'added_half_quads':42,
        'scope':'Continuous source arc sampling; original eye and outer boundary anchors preserved'}


def adapt(sources):
    out = dict(sources); shell = out['shell.py']
    old = '    front_count = len(shell_faces)'
    if shell.count(old) != 1: raise ValueError('Outer flow insertion changed')
    shell = shell.replace(old,
        '    shell_faces, outer_flow_report = build_outer_source_flow(verts,shell_faces,levels,cols,lateral,nominal,feature_flow_surface_point)\n'+old)
    old = '    chin_columns, rear_ids, outline_trace["vertex_ids"] = flow_chains[:3]'
    if shell.count(old) != 1: raise ValueError('Outer flow compact mapping changed')
    shell = shell.replace(old,old+'\n    for item in outer_flow_report["rows"] + [outer_flow_report["lower_transition"],outer_flow_report["upper_transition"]]:\n        item["vertex_ids"] = [flow_remap[i] for i in item["vertex_ids"]]')
    old = '        "semantic_rows": levels,'
    if shell.count(old) != 1: raise ValueError('Outer flow reporting changed')
    shell = shell.replace(old,'        "outer_source_flow": outer_flow_report,\n'+old)
    out['shell.py'] = shell
    out['face.py'] += '\n\n'+'\n\n'.join(inspect.getsource(f) for f in
        (outer_flow_q,outer_flow_interp,outer_flow_arc_stations,outer_flow_fan,build_outer_source_flow))+'\n'
    return out
