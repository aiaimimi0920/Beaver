"""Shape-preserving fairing of the two nasal arcs, with the apex pinned."""

def hermite(u,z0,z1,m0,m1,length):
    return (2*u**3-3*u*u+1)*z0+(u**3-2*u*u+u)*length*m0+(-2*u**3+3*u*u)*z1+(u**3-u*u)*length*m1

def fit_arc(points,m0,m1):
    lo,z0=points[0];hi,z1=points[-1];length=hi-lo
    aa=ab=bb=ar=br=0.0
    for y,z in points[1:-1]:
        u=(y-lo)/length;g=u*u*(1-u)**2;k=g*u
        r=z-hermite(u,z0,z1,m0,m1,length)
        aa+=g*g;ab+=g*k;bb+=k*k;ar+=g*r;br+=k*r
    det=aa*bb-ab*ab
    assert det>1e-12
    return (lo,hi,z0,z1,m0,m1,(ar*bb-br*ab)/det,(br*aa-ar*ab)/det)

def evaluate_arc(y,arc):
    lo,hi,z0,z1,m0,m1,a,b=arc;u=max(0.0,min(1.0,(y-lo)/(hi-lo)))
    return hermite(u,z0,z1,m0,m1,hi-lo)+u*u*(1-u)**2*(a+b*u)

def linear_depth(y,points):
    for (a,za),(b,zb) in zip(points,points[1:]):
        if y<=b:return za+(zb-za)*(y-a)/(b-a)
    return points[-1][1]

def fair_nasal_arcs(data):
    from face import H,Y0
    from nose_refine import bump
    old=[v.co.copy() for v in data.vertices];normals=[p.normal.copy() for p in data.polygons]
    regions=data.attributes['NormalRegion']
    skin={i for p in data.polygons if regions.data[p.index].value==1 for i in p.vertices}
    center={}
    for i in skin:
        v=data.vertices[i];h=(v.co.z+.021-Y0)/H
        if abs(v.co.x)<1e-6 and .20<h<.43 and -v.co.y>.085:
            center[v.co.z]=max(center.get(v.co.z,0),-v.co.y)
    points=sorted(center.items());peak=max(range(len(points)),key=lambda i:points[i][1])
    assert 4<=peak<=len(points)-4
    lower=fit_arc(points[:peak+1],0.0,.2)
    upper=fit_arc(points[peak:],-1.35,-.12)
    for arc,sign in [(lower,1),(upper,-1)]:
        values=[evaluate_arc(arc[0]+(arc[1]-arc[0])*i/200,arc) for i in range(201)]
        assert all(sign*(b-a)>-1e-10 for a,b in zip(values,values[1:])), 'Nose arc reverses direction'
    lo,hi=points[0][0],points[-1][0];apex=points[peak][0];chosen=[];maximum=0.0
    for i in skin:
        v=data.vertices[i];y=v.co.z
        if not lo<y<hi or abs(y-apex)<1e-8 or abs(v.co.x)>=.017:continue
        arc=lower if y<apex else upper
        strength=.35 if y<apex else 1.0
        dz=(evaluate_arc(y,arc)-linear_depth(y,points))*bump(v.co.x/.017)*strength
        if abs(dz)<1e-12:continue
        v.co.y-=dz;chosen.append(i);maximum=max(maximum,abs(dz))
    assert chosen and maximum<.0008, 'Nose fairing exceeds shape-preserving bound'
    data.update();selected=set(chosen)
    assert all((v.co-p).length<1e-9 for v,p in zip(data.vertices,old) if v.index not in selected)
    for p,n in zip(data.polygons,normals):
        if any(i in selected for i in p.vertices):assert p.area>1e-12 and p.normal.dot(n)>.85
    return {'kind':'two quintic arcs with fixed endpoints and apex','selected_vertices':len(chosen),'max_delta_m':maximum,'apex_blender_z':apex,'apex_depth_m':points[peak][1],'lower_arc':lower,'upper_arc':upper,'outside_nose_unchanged':True}
