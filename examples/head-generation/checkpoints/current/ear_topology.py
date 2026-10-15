"""Improve one internal lower-root diagonal per ear, without moving vertices."""
def triangle_quality(a,b,c):
    from math import sqrt
    u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)]
    cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    den=sum((p[i]-q[i])**2 for p,q in ((a,b),(b,c),(c,a)) for i in range(3))
    return 2*sqrt(3)*sqrt(sum(x*x for x in cross))/max(den,1e-20)

def improve_lower_root_diagonals(data,reports):
    import bmesh
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    old=[v.co.copy() for v in data.vertices]
    bm=bmesh.new();bm.from_mesh(data);bm.verts.ensure_lookup_table();bm.normal_update()
    region=bm.faces.layers.int.get('NormalRegion');assert region is not None
    tree=KDTree(len(bm.verts))
    for i,v in enumerate(bm.verts):tree.insert(v.co,i)
    tree.balance();results=[]
    for report in reports:
        p=min(report['root_positions'],key=lambda p:p[2])
        _,i,error=tree.find(Vector((p[0],p[1],p[2]-.021)));assert error<.000002
        root=bm.verts[i];candidates=[]
        for edge in root.link_edges:
            faces=list(edge.link_faces)
            if len(faces)!=2 or any(len(f.verts)!=3 for f in faces):continue
            if not (40<=faces[0][region]<50 and faces[0][region]==faces[1][region]):continue
            a,b=edge.verts
            c=next(v for v in faces[0].verts if v not in (a,b))
            d=next(v for v in faces[1].verts if v not in (a,b))
            if c==d or bm.edges.get((c,d)) is not None:continue
            if max((v.co-root.co).length for v in (a,b,c,d))>.012:continue
            q0=min(triangle_quality(*[v.co for v in f.verts]) for f in faces)
            q1=min(triangle_quality(a.co,c.co,d.co),triangle_quality(b.co,c.co,d.co))
            if q1<=q0+.05:continue
            normal=(faces[0].normal+faces[1].normal).normalized()
            # The new diagonal must split a convex projected quadrilateral.
            side_a=(d.co-c.co).cross(a.co-c.co).dot(normal)
            side_b=(d.co-c.co).cross(b.co-c.co).dot(normal)
            if side_a*side_b>=-1e-18:continue
            candidates.append((q1-q0,edge,q0,q1,normal.copy()))
        assert candidates,'No safe lower-root quality improvement found'
        _,edge,q0,q1,normal=max(candidates,key=lambda x:x[0])
        rotated=bmesh.utils.edge_rotate(edge,False);assert rotated is not None
        bm.normal_update()
        assert all(f.calc_area()>1e-12 and f.normal.dot(normal)>.1 for f in rotated.link_faces),'Rotated root face inverted'
        actual=min(triangle_quality(*[v.co for v in f.verts]) for f in rotated.link_faces)
        assert actual>q0+.05
        results.append({'before_quality':q0,'after_quality':actual,'side':1 if root.co.x>0 else -1})
    assert len(results)==2
    bm.to_mesh(data);bm.free();data.update()
    assert len(data.vertices)==len(old) and all((v.co-p).length<1e-9 for v,p in zip(data.vertices,old))
    return {'edge_flips':results,'all_vertex_positions_unchanged':True}
