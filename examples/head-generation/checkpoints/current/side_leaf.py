"""Independent folded secondary lashes embedded in a smooth lateral parent."""

def leaf_sections(start, peak, end, reach, embed):
    assert 0 <= start < peak < end <= 1 and reach > 0 and embed > 0
    import math
    rows=[]
    for f in [start,(start+peak)/2,peak,(peak+end)/2,end]:
        w=(f-start)/(peak-start) if f<=peak else (end-f)/(end-peak)
        lower=-embed*max(0,math.sin(math.pi*(f-start)/(end-start)))**.6
        rows.append((f,lower,lower+(reach-lower)*max(0,w)))
    if all(abs(f-peak)>1e-10 for f,_,_ in rows):rows.append((peak,-embed*math.sin(math.pi*(peak-start)/(end-start))**.6,reach))
    return sorted(rows)


def build_side_leaves(parts, parent_sections, style):
    from face import H
    from thin_ribbon import accent_mesh
    from lateral_fit import interpolate_shift
    from mouth import store_mouth_deltas
    for side in (-1,1):
        parent=next(o for o in parts if o.name=='Outer lateral eye liner '+str(side))
        bases=[];stations=[];offset=0
        for f,lo,hi in parent_sections:
            count=1 if hi-lo<1e-9 else 6
            ring=list(parent.data.vertices)[offset:offset+count];offset+=count
            base=ring[0].co if count==1 else ring[3].co
            bases.append(tuple(base));stations.append(f)
        assert offset==len(parent.data.vertices)
        profile=style['lateral_accent_profile']
        peaks=[i for i in range(1,len(profile)-1) if profile[i][1]>profile[i-1][1] and profile[i][1]>profile[i+1][1]]
        assert len(peaks)==2
        for number,index in enumerate(peaks):
            start=profile[index-1][0];peak,reach=profile[index];end=profile[index+1][0]
            start=peak+(start-peak)*.7;end=peak+(end-peak)*.7
            sections=leaf_sections(start,peak,end,reach,.0006/H)
            def point(f,offset,relief):
                base=[interpolate_shift(f,stations,[p[k] for p in bases]) for k in range(3)]
                outward=max(0,offset*H)
                return (base[0]+side*offset*H,base[1]-.00012-relief+.7*outward,base[2]-.15*outward)
            obj=accent_mesh('Embedded lateral child '+str(number)+' '+str(side),sections,point,
                tuple(style['accent_outer_tint_rgb']),.00022,.001,True)
            obj['side_leaf_parent']=parent.name
            # Lower boundary of every noncollapsed section is embedded.
            roots=[];cursor=0
            for _,lo,hi in sections:
                count=1 if hi-lo<1e-9 else 6
                roots.extend([cursor] if count==1 else [cursor,cursor+1]);cursor+=count
            obj['side_leaf_root_indices']=roots
            store_mouth_deltas(obj)


def measure_side_leaves(parts):
    from mathutils.bvhtree import BVHTree
    result={}
    for obj in parts:
        if not obj.get('side_leaf_parent'):continue
        parent=next(p for p in parts if p.name==obj['side_leaf_parent'])
        tree=BVHTree.FromPolygons([v.co.copy() for v in parent.data.vertices],[tuple(p.vertices) for p in parent.data.polygons])
        distances=[tree.find_nearest(obj.data.vertices[i].co)[3] for i in obj['side_leaf_root_indices']]
        assert max(distances)<.0008, 'Detached side child root: '+obj.name
        assert obj.data != parent.data
        result[obj.name]={'parent':parent.name,'independent_mesh':True,'root_samples':len(distances),'maximum_root_distance_m':max(distances),'root_limit_m':.0008}
    assert len(result)==4
    return result


def stable_part_order(parts):
    return [p for p in parts if not p.get("side_leaf_parent")]+[p for p in parts if p.get("side_leaf_parent")]
