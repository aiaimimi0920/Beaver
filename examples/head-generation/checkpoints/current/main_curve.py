"""Curved thin-return sections derived only from the original generated mesh."""

def curved_section(low, crest, high, thickness, profile="rounded", soft_curve=0.0):
    assert profile in ("rounded", "creased")
    assert 0.0<=soft_curve<=1.0
    assert high[2]>crest[2]>low[2] and thickness>0
    q=(crest[2]-low[2])/(high[2]-low[2])
    baseline=low[1]*(1-q)+high[1]*q
    relief=baseline-crest[1]
    front=[]
    for f in (0.0,q/2,q,(q+1)/2,1.0):
        weight=(f/q if f<=q else (1-f)/(1-q)) if profile=="creased" else (1-(1-f/q)**2 if f<=q else 1-((f-q)/(1-q))**2)
        if profile=="creased":
            curved=1-(1-f/q)**2 if f<=q else 1-((f-q)/(1-q))**2
            weight=weight*(1-soft_curve)+curved*soft_curve
        front.append((low[0],low[1]*(1-f)+high[1]*f-relief*weight,low[2]*(1-f)+high[2]*f))
    back=[(x,y+thickness,z) for x,y,z in front]
    return [back[0],*front,*reversed(back[1:])]


def curved_mesh(columns):
    vertices=[];rings=[];faces=[];zones=[]
    for column in columns:
        rings.append(list(range(len(vertices),len(vertices)+len(column))))
        vertices.extend(column)
    for a,b in zip(rings,rings[1:]):
        assert len(a) in (1,10) and len(b) in (1,10) and len(a)+len(b)>2
        for lane in range(10):
            nxt=(lane+1)%10
            if len(a)==1: face=(a[0],b[nxt],b[lane])
            elif len(b)==1:face=(a[lane],a[nxt],b[0])
            else:face=(a[lane],a[nxt],b[nxt],b[lane])
            zone=1 if lane in (1,2) else 2 if lane in (3,4) else 0
            if len(face)==4:
                if lane>=6:face=face[1:]+face[:1]
                faces.extend([(face[0],face[1],face[2]),(face[0],face[2],face[3])]);zones.extend([zone,zone])
            else:faces.append(face);zones.append(zone)
    for ring,reverse in ((rings[0],True),(rings[-1],False)):
        if len(ring)>1:faces.append(tuple(reversed(ring)) if reverse else tuple(ring));zones.append(0)
    return vertices,faces,zones


def refine_main_cross_sections(parts):
    from face import mesh,bpy,STYLE
    from thin_ribbon import signed_volume
    report={}
    for obj in list(parts):
        if not obj.name.startswith('Upper lash '):continue
        rows={}
        for v in obj.data.vertices:rows.setdefault(round(abs(v.co.x),8),[]).append(v)
        columns=[];max_relief=0
        for x,column in sorted(rows.items()):
            if len(column)==1:columns.append([tuple(column[0].co)]);continue
            assert len(column)==6
            levels=sorted(set(round(v.co.z,7) for v in column))
            assert len(levels)==3
            groups=[sorted([v for v in column if abs(v.co.z-z)<1e-7],key=lambda v:v.co.y) for z in levels]
            assert all(len(g)==2 for g in groups)
            lo,crest,hi=[tuple(g[0].co) for g in groups]
            thickness=sum(g[1].co.y-g[0].co.y for g in groups)/3
            columns.append(curved_section(lo,crest,hi,thickness,STYLE.get("accent_cross_section_profile","rounded"),STYLE.get("accent_section_soft_curve",0.0)))
        vertices,faces,zones=curved_mesh(columns)
        if signed_volume(vertices,faces)<0:faces=[(f[0],*reversed(f[1:])) for f in faces]
        volume=signed_volume(vertices,faces);assert volume>1e-14
        color=tuple(obj.data.color_attributes['Color'].data[0].color)[:3]
        temporary=mesh('Main curved section temporary',vertices,faces,color)
        obj.data=temporary.data
        parts.remove(temporary);bpy.data.objects.remove(temporary,do_unlink=True)
        attr=obj.data.attributes.new('AccentNormalZone','INT','FACE')
        front=[]
        for poly,zone in zip(obj.data.polygons,zones):
            attr.data[poly.index].value=zone;poly.use_smooth=zone in (1,2)
            if zone in (1,2):front.append(poly.index)
        obj['accent_front_faces']=front;obj['accent_positive_volume_m3']=volume
        report[obj.name]={'stations':len(columns),'noncollapsed_section_vertices':10,'original_boundaries_and_crest_preserved':True}
    return report


def is_collinear_refinement(point,a,b,fraction,tolerance=1e-7):
    return sum((point[i]-(a[i]*(1-fraction)+b[i]*fraction))**2 for i in range(3))<=tolerance*tolerance
