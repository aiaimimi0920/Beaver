"""Strict nonadjacent skin segment/triangle crossings; not a full collision proof."""
import inspect


def strict_skin_crossings(vertices, triangles):
    import numpy as np
    v=np.asarray(vertices,dtype=float)
    f=np.asarray(triangles,dtype=int)
    if not np.isfinite(v).all() or f.ndim!=2 or f.shape[1]!=3:
        raise ValueError('Expected finite coordinates and triangular skin')
    t=v[f]
    low,high=t.min(axis=1),t.max(axis=1)
    pairs=[]
    for i,a in enumerate(f):
        candidates=np.where((low[:i]<=high[i]+1e-12).all(axis=1)
                            &(high[:i]>=low[i]-1e-12).all(axis=1))[0]
        for j in candidates:
            b=f[j]
            if set(a)&set(b):
                continue
            hit=False
            for segment_ids,triangle_ids in ((a,b),(b,a)):
                p0,p1,p2=v[triangle_ids]
                e1,e2=p1-p0,p2-p0
                for k in range(3):
                    start=v[segment_ids[k]]
                    direction=v[segment_ids[(k+1)%3]]-start
                    h=np.cross(direction,e2)
                    determinant=e1@h
                    if abs(determinant)<1e-14:
                        continue
                    s=start-p0
                    u=s@h/determinant
                    r=np.cross(s,e1)
                    w=direction@r/determinant
                    distance=e2@r/determinant
                    if (1e-8<u and 1e-8<w and u+w<1-1e-8
                            and 1e-8<distance<1-1e-8):
                        hit=True
            if hit:
                pairs.append((int(i),int(j)))
    return pairs


def reject_skin_crossings(data):
    regions=data.attributes.get('NormalRegion')
    if regions is None:
        raise ValueError('Missing skin region labels')
    polygons=[p for p in data.polygons if regions.data[p.index].value in (1,2)]
    pairs=strict_skin_crossings([tuple(v.co) for v in data.vertices],
                               [tuple(p.vertices) for p in polygons])
    if pairs:
        actual=[(polygons[a].index,polygons[b].index) for a,b in pairs]
        raise ValueError('Nonadjacent skin crossings: '+str(actual[:16]))
    return {'checked_triangles':len(polygons),'strict_crossings':0,
            'scope':'neutral nonadjacent strict segment crossings; excludes coplanar overlaps, shared-vertex pairs and posed collision proof'}


def verify_export_quad_cells(data,source_quads):
    lookup={frozenset(p.vertices):p.normal.copy() for p in data.polygons}
    minimum=1.0
    for source_id,quad in source_quads:
        choices=(((0,1,2),(0,2,3)),((0,1,3),(1,2,3)))
        actual=[]
        for first,second in choices:
            a=frozenset(quad[i] for i in first);b=frozenset(quad[i] for i in second)
            if a in lookup and b in lookup:actual.append(float(lookup[a].dot(lookup[b])))
        if len(actual)!=1:
            raise ValueError('Missing/ambiguous source quad export: '+str(source_id))
        minimum=min(minimum,actual[0])
        if actual[0]<-1e-7:
            raise ValueError('Automatic export folds source quad: '+str((source_id,quad,actual[0])))
    return {'checked_source_skin_quads':len(source_quads),'minimum_actual_pair_dot':minimum,
            'scope':'actual automatic diagonal; not an all-diagonals or all-poses proof'}


def adapt(sources):
    out=dict(sources)
    out['ear_surface.py']+='\n\n'+inspect.getsource(strict_skin_crossings)+'\n\n'+inspect.getsource(reject_skin_crossings)+'\n\n'+inspect.getsource(verify_export_quad_cells)
    anchor="        report['automatic_export_triangle_verification'] = verify_skin_triangles(data)"
    assert out['pipeline.py'].count(anchor)==1
    out['pipeline.py']=out['pipeline.py'].replace(anchor,anchor+"\n        from ear_surface import reject_skin_crossings\n        report['neutral_skin_crossings'] = reject_skin_crossings(data)\n        from ear_surface import verify_export_quad_cells\n        report['automatic_quad_cells'] = verify_export_quad_cells(data,_source_skin_quads)",1)
    return out
