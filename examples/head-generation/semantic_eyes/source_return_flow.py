"""Candidate boundary-normal posterior strip with explicit finite open ends."""
import inspect


def feature_return_offsets(points, edges, config, height, origin, depth_origin):
    from math import hypot, sqrt
    neighbors = {v:[] for v in points}
    for a,b in edges:
        neighbors[a].append(b);neighbors[b].append(a)
    if any(len(ns) not in (1,2) for ns in neighbors.values()):
        raise ValueError('Posterior source boundary branches')
    result = {}
    center_height = origin+.5*height-.021
    for v,point in points.items():
        ns = neighbors[v]
        if len(ns)==2:
            tangent = (points[ns[1]][0]-points[ns[0]][0],
                       points[ns[1]][2]-points[ns[0]][2])
        else:
            tangent = (points[ns[0]][0]-point[0],points[ns[0]][2]-point[2])
        length = hypot(*tangent)
        if length <= 1e-9:
            raise ValueError('Posterior boundary projection has no stable tangent')
        nx,nz = -tangent[1]/length,tangent[0]/length
        if nx*(-point[0])+nz*(center_height-point[2])<0:
            nx,nz = -nx,-nz
        edge_vectors = [(points[n][0]-point[0],points[n][2]-point[2]) for n in ns]
        spans = [hypot(*edge) for edge in edge_vectors]
        if min(spans)<=1e-9:
            raise ValueError('Posterior boundary has a collapsed projected segment')
        maximum = max(config['ear_post_return_width_m'],config['ear_post_lower_return_width_m'])
        if len(ns)==2:
            cosine = sum(a*b for a,b in zip(*edge_vectors))/(spans[0]*spans[1])
            half_turn_sine = sqrt(max(0.,min(1.,(1+cosine)*.5)))
            if half_turn_sine>1e-8:
                maximum = min(maximum,.45*min(spans)/half_turn_sine)
        if abs(point[0])>1e-8 and point[0]*nx<0:
            maximum = min(maximum,.8*abs(point[0])/abs(nx))
        h = (point[2]+.021-origin)/height
        upper,lower = return_weight(h),lower_return_weight(h)
        desired = config['ear_post_return_width_m']*upper+config['ear_post_lower_return_width_m']*lower
        width = min(maximum,max(desired,.18*max(spans),.0005))
        if width <= 1e-6:
            raise ValueError('Posterior source width collapses under boundary constraints')
        depth = config['ear_post_return_depth_m']*max(upper,lower,.25)
        result[v] = (nx*width,depth,nz*width)
    return result


def adapt(sources):
    out = dict(sources)
    source = out['ear_return.py']
    if source.count('    for fraction in (.5,1.0):') != 1:
        raise ValueError('Posterior row sampling contract changed')
    source = source.replace('    for fraction in (.5,1.0):','    for fraction in (1.0,):',1)
    marker = '    ids = sorted({v for edge in selected for v in edge.verts}, key=original_index.__getitem__)'
    if source.count(marker) != 1:
        raise ValueError('Posterior boundary ownership contract changed')
    source = source.replace(marker,marker+'\n    offsets = feature_return_offsets({v:tuple(v.co) for v in ids},[tuple(e.verts) for e in selected],C,H,Y0,Z0)',1)
    start = source.index('            height = (v.co.z+.021-Y0)/H')
    end = source.index('            new = bm.verts.new(point, v)',start)
    source = source[:start]+'''            dx,dy,dz = offsets[v]
            point = v.co.copy()
            point.x += dx*fraction
            point.y += dy*sin(pi*fraction/2)
            point.z += dz*fraction
            if abs(v.co.x)<1e-8:
                point.x = 0.0
            if (point-v.co).length<1e-7:
                raise ValueError('Posterior source row unexpectedly collapsed')
'''+source[end:]
    marker = '    assert all(face.calc_area()>1e-12 for face in new_faces),'
    if source.count(marker) != 1:
        raise ValueError('Posterior source validation stage changed')
    source = source.replace(marker,"    assert all(len(face.verts)==4 for face in new_faces), 'Posterior source must retain finite quad ends'\n"+marker,1)
    out['ear_return.py'] = source+'\n\n'+inspect.getsource(feature_return_offsets)
    return out
