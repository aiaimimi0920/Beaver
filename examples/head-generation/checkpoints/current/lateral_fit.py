"""Final lateral-lash guide and section fit on the original generated skin."""
import math

def depth_shifts(stations, centers):
    assert len(stations) == len(centers) and len(stations) > 1
    assert stations[0] == 0 and stations[-1] == 1
    first, last = centers[0], centers[-1]
    result = [first*(1-f)+last*f-current for f,current in zip(stations,centers)]
    assert max(abs(d) for d in result) < .004, "Lateral depth correction exceeds design bound"
    return result

def interpolate_shift(f, stations, shifts):
    if f <= stations[0]: return shifts[0]
    for i in range(1,len(stations)):
        if f <= stations[i]:
            u=(f-stations[i-1])/(stations[i]-stations[i-1])
            return shifts[i-1]*(1-u)+shifts[i]*u
    return shifts[-1]

def regularize_lateral_sections(parts, sections, style, face_tree):
    report={}
    stations=[s[0] for s in sections]
    for side in (-1,1):
        obj=next(o for o in parts if o.name == "Outer lateral eye liner "+str(side))
        rings=[]; offset=0
        for _,lo,hi in sections:
            count=1 if hi-lo < 1e-9 else 6
            rings.append(list(obj.data.vertices)[offset:offset+count]); offset+=count
        assert offset == len(obj.data.vertices), "Unexpected lateral section layout"
        centers=[r[0].co.y if len(r)==1 else (r[1].co.y+r[3].co.y)/2 for r in rings]
        shifts=depth_shifts(stations,centers)
        for ring,shift in zip(rings,shifts):
            for vertex in ring: vertex.co.y+=shift
        # Seat the base guide, not the free teeth, against the final own face.
        # A frontal depth ray misses the aperture boundary; nearest own-face
        # support supplies the needed lateral as well as depth displacement.
        support_moves=[]
        rotations=[]; seated_bases=[]
        for (f,lo,hi),ring in zip(sections,rings):
            if len(ring)==1:
                base=ring[0].co.copy()
            else:
                fraction=max(0.0,min(1.0,-lo/max(1e-9,hi-lo)))
                base=ring[1].co.lerp(ring[3].co,fraction)
            hit,normal,_,distance=face_tree.find_nearest(base)
            assert hit is not None and distance < .012, "Missing own-face lateral support"
            delta=seating_delta(tuple(base),tuple(hit),tuple(normal),f,style["accent_evaluated_clearance_m"])
            for vertex in ring:
                for axis in (0,1): vertex.co[axis]+=delta[axis]
            support_moves.append(delta)
            seated_base=tuple(base[i]+delta[i] for i in range(3))
            angle=0.0
            if len(ring)>1:
                tangent=tuple(ring[3].co[i]-ring[1].co[i] for i in range(3))
                angle=section_rotation(tangent,tuple(normal),side,f)
                for vertex in ring:
                    vertex.co=rotate_section(tuple(vertex.co),seated_base,angle)
            rotations.append(angle); seated_bases.append(seated_base)
        # Closest-skin normals may jump across face triangles. Keep the
        # section orientation continuous between its own attached endpoints.
        valid=[i for i,r in enumerate(rings) if len(r)>1]
        measured={i:math.atan2(rings[i][3].co.y-rings[i][1].co.y,
                               side*(rings[i][3].co.x-rings[i][1].co.x)) for i in valid}
        angles=continuous_section_angles(stations,measured[valid[0]],measured[valid[-1]],stations[valid[-1]])
        angles=front_readable_frames(stations,angles,style.get("accent_lateral_front_bias",0.0))
        frame_corrections=[]
        for i,ring in enumerate(rings):
            correction=side*(angles[i]-measured[i]) if i in measured else 0.0
            for vertex in ring:
                vertex.co=rotate_section(tuple(vertex.co),seated_bases[i],correction)
            rotations[i]+=correction
            frame_corrections.append(correction)
        guide_moves=continuous_guide_moves(stations,seated_bases,style["accent_lateral_guide_bow_fraction"],style["accent_lateral_guide_inward_power"])
        # Preserve the smooth inner silhouette while tapering the outer body.
        # Both main and companion use the same translated guide below.
        anchor=style.get("accent_lateral_taper_anchor_profile")
        if anchor:
            for i,((f,lo,hi),ring) in enumerate(zip(sections,rings)):
                if len(ring)==1: continue
                width=style["calibration"]["outer_liner_width"]
                old_lo=-width*sample_profile(f,anchor)
                correction=taper_anchor_shift(tuple(ring[1].co),tuple(ring[3].co),lo,hi,old_lo)
                guide_moves[i]=tuple(guide_moves[i][k]+correction[k] for k in range(3))
        if style.get("accent_lateral_eye_rim_follow",False):
            lo0,hi0=sections[0][1:]
            first=rings[0][1].co.lerp(rings[0][3].co,-lo0/(hi0-lo0));last=rings[-1][0].co
            endpoints=((first.x+guide_moves[0][0],first.z),(last.x+guide_moves[-1][0],last.z))
            for i,((f,lo,hi),ring) in enumerate(zip(sections,rings)):
                fraction=max(0.0,min(1.0,-lo/max(1e-9,hi-lo)))
                # The outer baseline seats on the rim; ribbon width overlays
                # inward across the rim. Aligning its inner edge would widen
                # the visible eye opening and remove its eyeliner coverage.
                base=ring[0].co if len(ring)==1 else ring[1].co.lerp(ring[3].co,fraction)
                current=base.x+guide_moves[i][0]
                shift=eye_rim_follow_shift(current,base.z,side,f,style["calibration"],endpoints)
                guide_moves[i]=(guide_moves[i][0]+shift,guide_moves[i][1],guide_moves[i][2])
        for ring,delta in zip(rings,guide_moves):
            for vertex in ring:
                for axis in (0,1): vertex.co[axis]+=delta[axis]
        obj.data.update()
        companion=next(o for o in parts if o.name == "Outer companion lash "+str(side))
        offset=0
        for i in range(17):
            count=1 if i in (0,16) else 6
            f=style["accent_lateral_branch_start"]+style["accent_lateral_branch_span"]*i/16
            shift=interpolate_shift(f,stations,shifts)
            for vertex in list(companion.data.vertices)[offset:offset+count]:
                vertex.co.y+=shift
                for axis in (0,1):
                    vertex.co[axis]+=interpolate_shift(f,stations,[d[axis] for d in support_moves])
            base=tuple(interpolate_shift(f,stations,[b[axis] for b in seated_bases]) for axis in range(3))
            angle=interpolate_shift(f,stations,rotations)
            for vertex in list(companion.data.vertices)[offset:offset+count]:
                vertex.co=rotate_section(tuple(vertex.co),base,angle)
                for axis in (0,1):
                    vertex.co[axis]+=interpolate_shift(f,stations,[d[axis] for d in guide_moves])
            offset+=count
        assert offset == len(companion.data.vertices)
        parent_bases=[]
        for (f,lo,hi),ring in zip(sections,rings):
            fraction=max(0.0,min(1.0,-lo/max(1e-9,hi-lo)))
            parent_bases.append(tuple(ring[0].co if len(ring)==1 else ring[1].co.lerp(ring[3].co,fraction)))
        paired_shifts=[];offset=0
        for i in range(17):
            count=1 if i in (0,16) else 6
            f=style["accent_lateral_branch_start"]+style["accent_lateral_branch_span"]*i/16
            ring=list(companion.data.vertices)[offset:offset+count]
            edge=ring[0].co if count==1 else ring[1].co
            target=tuple(interpolate_shift(f,stations,[b[k] for b in parent_bases]) for k in range(3))
            delta=paired_layer_delta(tuple(edge),target,side)
            assert math.dist((0,0,0),delta)<.004, 'Companion/parent guide mismatch exceeds local correction'
            for vertex in ring:
                for axis in (0,1,2):vertex.co[axis]+=delta[axis]
            # Independent slim companion curve: endpoint-guided, not a copy
            # of the eye rim. Both ends converge while the middle opens gently.
            phase=i/16
            start=style["accent_lateral_branch_start"]
            end=start+style["accent_lateral_branch_span"]
            ends=[tuple(interpolate_shift(g,stations,[b[k] for b in parent_bases]) for k in range(3)) for g in (start,end)]
            low=companion_curve(ends[0],ends[1],side,phase)
            width=style['accent_lateral_branch_width_H']*style['calibration']['height_m']*math.sin(math.pi*phase)**.8
            if count==1:
                ring[0].co=low
            else:
                for vertex,(u,back,ridge) in zip(ring,[(0,1,0),(0,0,0),(.42,0,1),(1,0,0),(1,1,0),(.42,1,1)]):
                    vertex.co=(low[0]+side*width*u,low[1]+.35*width*u+back*style['accent_lateral_thickness_m']-.00006*ridge,low[2])
            paired_shifts.append(math.dist((0,0,0),delta));offset+=count
        companion.data.update()
        report[str(side)]={"maximum_companion_parent_pairing_m":max(paired_shifts),"maximum_section_shift_m":max(abs(d) for d in shifts),"root_preserved":True,"maximum_frame_correction_degrees":max(abs(a)*180/math.pi for a in frame_corrections),"maximum_support_seating_m":max(sum(v*v for v in d)**.5 for d in support_moves),"cross_section_preserved":True,"maximum_final_guide_correction_m":max(math.dist((0,0,0),d) for d in guide_moves),"maximum_section_rotation_degrees":max(abs(a)*180/math.pi for a in rotations)}
    return report


def seating_delta(base, hit, normal, phase, clearance):
    u=max(0.0,min(1.0,phase/.25))
    weight=u*u*(3-2*u)
    result=tuple((hit[i]+normal[i]*clearance-base[i])*weight if i<2 else 0.0 for i in range(3))
    assert sum(d*d for d in result)**.5 < .004, "Lateral seating requires structural redesign"
    return result


def section_rotation(tangent, normal, side, phase):
    """Turn a cross-section toward the local skin tangent without flattening it."""
    if math.hypot(tangent[0],tangent[1])<1e-10 or math.hypot(normal[0],normal[1])<1e-10:
        return 0.0
    target=(-normal[1],normal[0])
    if target[0]*side<0: target=(-target[0],-target[1])
    angle=math.atan2(tangent[0]*target[1]-tangent[1]*target[0],tangent[0]*target[0]+tangent[1]*target[1])
    angle=max(-math.pi/4,min(math.pi/4,angle))
    u=max(0.0,min(1.0,phase/.3));return angle*u*u*(3-2*u)


def rotate_section(point, base, angle):
    dx,dy=point[0]-base[0],point[1]-base[1]
    c,s=math.cos(angle),math.sin(angle)
    return (base[0]+c*dx-s*dy,base[1]+s*dx+c*dy,point[2])


def continuous_guide_moves(stations,bases,bow_fraction=1.0,inward_power=2.0):
    assert 1.0<=inward_power<=4.0
    assert 0<=bow_fraction<=1
    assert len(stations)==len(bases) and stations[0]==0 and stations[-1]==1
    start,end=bases[0],bases[-1]
    weights=[4*f*(1-f) for f in stations]
    denominator=sum(w*w for w in weights)
    bow=sum(w*(base[1]-(start[1]*(1-f)+end[1]*f)) for f,w,base in zip(stations,weights,bases))/max(1e-12,denominator)
    moves=[]
    for f,base in zip(stations,bases):
        target=(start[0]+(end[0]-start[0])*f**inward_power,start[1]+(end[1]-start[1])*f+4*f*(1-f)*bow*bow_fraction,base[2])
        delta=tuple(target[i]-base[i] for i in range(3))
        assert math.dist((0,0,0),delta)<.004, 'Final lateral guide requires structural redesign'
        moves.append(delta)
    return moves


def wing_depth_shift(current_y, target_y, phase):
    u=max(0.0,min(1.0,phase/.6))
    delta=(target_y-current_y)*u*u*(3-2*u)
    assert abs(delta)<.004, 'Outer wing seating requires structural redesign'
    return delta


def seat_outer_wings(parts, style, face_tree):
    report={}
    for side in (-1,1):
        wing=next(o for o in parts if o.name=='Attached outer lash wing '+str(side))
        columns={}
        for v in wing.data.vertices:columns.setdefault(round(abs(v.co.x),8),[]).append(v)
        xs=sorted(columns);moves=[]
        for x in xs:
            col=columns[x]
            base=sum((v.co for v in col),col[0].co*0)/len(col)
            hit,normal,_,distance=face_tree.find_nearest(base)
            assert hit is not None and distance<.012, 'Missing outer wing skin support'
            phase=(x-xs[0])/max(1e-9,xs[-1]-xs[0])
            back=max(v.co.y for v in col)
            target=hit.y+normal.y*style['accent_evaluated_clearance_m']
            shift=wing_depth_shift(back,target,phase)
            for v in col:v.co.y+=shift
            moves.append(shift)
        # The overlapping upper rail shares the main ribbon's depth. A skin-only
        # seat leaves the two rails separated in depth and opens a long slit in
        # oblique views, even when the frontal silhouette looks joined.
        main=next(o for o in parts if o.name=='Upper lash '+str(side))
        maincols={}
        for v in main.data.vertices:maincols.setdefault(round(abs(v.co.x),8),[]).append(v)
        guides=[]
        for x,col in sorted(maincols.items()):
            z=min(v.co.z for v in col)
            guides.append((x,min(v.co.y for v in col if abs(v.co.z-z)<1e-7)))
        for x in xs:
            phase=(x-xs[0])/max(1e-9,xs[-1]-xs[0])
            col=columns[x];lo=min(v.co.z for v in col);hi=max(v.co.z for v in col)
            if hi-lo<1e-9 or x>guides[-1][0]:continue
            target=sample_profile(x,guides)-.0001
            top=min(v.co.y for v in col if abs(v.co.z-hi)<1e-7)
            shift=upper_wing_depth_blend(target-top,phase)
            for v in col:
                weight=max(0,min(1,(v.co.z-lo)/(hi-lo)))
                v.co.y+=shift*weight
        wing.data.update()
        report[str(side)]={'upper_rail_depth_shared':True,'root_preserved':True,'maximum_depth_seating_m':max(map(abs,moves)),'silhouette_xz_preserved':True}
    return report


def continuous_section_angles(stations, start, end, end_station):
    """Use own endpoint frames; preserve both without local normal jitter."""
    assert stations[0]==0 and 0<end_station<=1
    result=[]
    for f in stations:
        u=max(0.0,min(1.0,f/end_station))
        w=u*u*(3-2*u)
        result.append(start+(end-start)*w)
    return result


def front_readable_frames(stations,angles,bias):
    """Keep roots fixed; turn only middle side-lash sections gently toward frontal readability."""
    assert 0 <= bias <= .5
    return [angle*(1-bias*math.sin(math.pi*f)**2) for f,angle in zip(stations,angles)]


def paired_layer_delta(edge,target,side):
    """Share the parent guide, with a small front relief and overlapping edge."""
    return (target[0]-side*.00008-edge[0],target[1]-.0002-edge[1],target[2]-edge[2])


def sample_profile(y, points):
    axis=1
    xs = [p[0] for p in points]
    ys = [p[axis] for p in points]
    slopes = [(ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]) for i in range(len(xs) - 1)]
    ds = (
        [slopes[0]]
        + [0 if a * b <= 0 else 2 * a * b / (a + b) for a, b in zip(slopes, slopes[1:])]
        + [slopes[-1]]
    )
    i = next((i for i in range(len(xs) - 1) if y <= xs[i + 1]), len(xs) - 2)
    t = max(0, min(1, (y - xs[i]) / (xs[i + 1] - xs[i])))
    h = xs[i + 1] - xs[i]
    return (
        (2 * t**3 - 3 * t * t + 1) * ys[i]
        + (t**3 - 2 * t * t + t) * h * ds[i]
        + (-2 * t**3 + 3 * t * t) * ys[i + 1]
        + (t**3 - t * t) * h * ds[i + 1]
    )

def taper_anchor_shift(inner,outer,lo,hi,anchor_lo):
    factor=(anchor_lo-lo)/max(1e-9,hi-lo)
    delta=tuple((outer[k]-inner[k])*factor if k<2 else 0.0 for k in range(3))
    assert math.dist((0,0,0),delta)<.003, "Taper requires excessive silhouette correction"
    return delta


def eye_rim_x_at_height(z, calibration):
    c=calibration;h=c["height_m"];y0=c["origin_y_m"]
    corner=y0+h*(c["eye_corner_y"]+c["eye_corner_slope"])
    upper=z>=corner
    def level(t):
        arch=c["eye_upper_arch"] if upper else -c["eye_lower_arch"]
        power=c["eye_upper_power"] if upper else c["eye_lower_power"]
        horizontal=c["eye_horizontal_power"] if upper else c["eye_lower_horizontal_power"]
        return y0+h*(c["eye_corner_y"]+c["eye_corner_slope"]*t+arch*max(0,1-t**horizontal)**power)
    low,high=.4,1.0
    for _ in range(45):
        mid=(low+high)/2
        if (level(mid)>z)==upper:low=mid
        else:high=mid
    t=(low+high)/2
    return h*(c["eye_center_x"]+c["eye_half_width"]*t)

def eye_rim_follow_shift(current_x,z,side,phase,calibration,endpoints=None):
    # Keep the authored upper/lower liner joins. Interpolate their overlap
    # allowance across the real eye-rim curve, rather than an independent bow.
    rim=side*eye_rim_x_at_height(z,calibration)
    if endpoints:
        a,b=endpoints
        c=calibration
        corner=c["origin_y_m"]+c["height_m"]*(c["eye_corner_y"]+c["eye_corner_slope"])
        # The upper-band join sits above the aperture corner. Parameterize
        # the visible side ribbon along the descending lower/outer rim only;
        # crossing the hidden upper rim would introduce an outward S bulge.
        rim=side*eye_rim_x_at_height(corner*(1-phase)+b[1]*phase,c)
        start_rim=side*eye_rim_x_at_height(corner,c)
        end_rim=side*eye_rim_x_at_height(b[1],c)
        fraction=(rim-start_rim)/(end_rim-start_rim)
        target=a[0]+(b[0]-a[0])*fraction
    else:target=rim-side*.00025
    u=max(0.0,min(1.0,phase/.22));weight=u*u*(3-2*u)
    delta=(target-current_x)*(1.0 if endpoints else weight)
    assert abs(delta)<.004,("Eye rim attachment needs structural redesign",current_x,z,side,phase,target,delta)
    return delta


def upper_wing_depth_blend(delta,phase):
    """Local overlap adjustment with unchanged root and free tip."""
    a=max(0,min(1,phase/.2));b=max(0,min(1,(.85-phase)/.2))
    weight=(a*a*(3-2*a))*(b*b*(3-2*b))
    assert abs(delta)<.006, 'Upper wing depth mismatch needs redesign'
    return delta*weight


def companion_curve(a,b,side,t):
    """Slim free curve with endpoint attachment and a separate middle bow."""
    assert 0 <= t <= 1
    q=t*t*(1.65-.65*t)
    bow=math.sin(math.pi*t)**2
    return (a[0]+(b[0]-a[0])*q+side*(.0012*bow-.00008),
            a[1]+(b[1]-a[1])*t-.0002-.00035*bow,
            a[2]+(b[2]-a[2])*t)
