"""Original thin folded eye accents, independent of the protected eye surfaces."""
from face import *
from main_curve import refine_main_cross_sections
from main_flank import shape_lower_flanks
from tuft_fit import seat_tuft_roots_on_parent
from lateral_fit import regularize_lateral_sections, seat_outer_wings
from thin_ribbon import outline_sections, accent_sections, signed_volume, closed_ribbon_mesh, accent_mesh


def upper_band_bow(t):
    outline = STYLE["upper_lash_rails"]
    lo, hi = min(p[0] for p in outline), max(p[0] for p in outline)
    phase = max(0.0, min(1.0, (t - lo) / (hi - lo)))
    return STYLE["accent_band_bow_m"] * sin(pi * phase)


def main_band_roll(t):
    rails = STYLE["upper_lash_rails"]
    phase = max(0.0,min(1.0,(t-rails[0][0])/(rails[-1][0]-rails[0][0])))
    low = STYLE["accent_band_end_roll_factor"]
    high = STYLE["accent_band_center_roll_factor"]
    return STYLE["accent_band_roll_m"]*(low+(high-low)*sin(pi*phase))


def upper_lid_support(x, y):
    """Existing skin-lid four-lane surface; no changes to the accepted lid."""
    side = 1 if x >= 0 else -1
    # Free tips continue the local lid support, not the distant cheek surface.
    limit = H*(C["eye_center_x"]+C["eye_half_width"]*STYLE["accent_corner_support_t"])
    inner_limit = H*(C["eye_center_x"]-C["eye_half_width"]*STYLE["accent_corner_support_t"])
    x = side*max(inner_limit,min(abs(x),limit))
    t = max(-1.0, min(1.0, (abs(x)/H-C["eye_center_x"])/C["eye_half_width"]))
    _, rim_y = eye_contour(side, t, True)
    fade = max(0.0, 1-t*t)**.6
    offsets = [.008*H, .002*H, -H*C["upper_cover"]*.6, -H*C["upper_cover"]]
    relief = [0.0, .0003, -.0004, -.0012]
    heights = [rim_y+o*fade for o in offsets]
    depths = [(depth(x,h) if i<2 else depth(x,rim_y))+relief[i]*fade
              for i,h in enumerate(heights)]
    if abs(t) > .999 or y >= heights[0]:
        return depth(x,y)
    for i in range(3):
        if y >= heights[i+1]:
            blend = (heights[i]-y)/max(1e-9,heights[i]-heights[i+1])
            return depths[i]*(1-blend)+depths[i+1]*blend
    return depths[-1]


def upper_attached_depth(x, y, thickness):
    baseline = upper_lid_support(x,y)
    evaluated_support = globals().get("_EVALUATED_LID_SUPPORT")
    if evaluated_support is not None:
        side = 1 if x >= 0 else -1
        reach = C["eye_half_width"]*STYLE["accent_corner_support_t"]
        sample_x = side*H*max(C["eye_center_x"]-reach,min(abs(x)/H,C["eye_center_x"]+reach))
        # Use the actual station inside the authored upper-band domain. Clamp
        # only extrapolated leaves, and reject distant rear/interior surfaces.
        station = (abs(x)/H-C["eye_center_x"])/C["eye_half_width"]
        rails = STYLE["upper_lash_rails"]
        # Extend continuously from the authored endpoint. Switching back to
        # the narrower analytic corner at t=1.2 made a false blunt wall.
        supported_t = max(rails[0][0],min(station,rails[-1][0]))
        probe_x = side*H*(C["eye_center_x"]+C["eye_half_width"]*supported_t)
        hit,_,_,_=evaluated_support.ray_cast(Vector((probe_x,-.5,y)),Vector((0,1,0)),1.0)
        if hit is not None and abs(-hit.y-baseline) <= STYLE["accent_evaluated_support_max_delta_m"]:
            baseline = -hit.y
    return baseline+thickness+STYLE["accent_surface_clearance_m"]


def main_ridge_relief(t, rise):
    lo = curve(t,1,STYLE["upper_lash_rails"])
    hi = curve(t,2,STYLE["upper_lash_rails"])
    if hi-lo < 1e-9:
        return 0.0
    u = max(0.0,min(1.0,(rise-lo)/(hi-lo)))
    peak = STYLE["accent_fold_ridge_fraction"]
    triangle = u/peak if u <= peak else (1-u)/(1-peak)
    return triangle*main_band_roll(t)*min(1.0,(hi-lo)/.014)


def tuft_rear_scoop(progress, maximum):
    """A rooted leaf retreats behind its parent before its free tip curls forward."""
    p=max(0.0,min(1.0,progress))
    return maximum*sin(pi*p/.45)**2 if 0.0 < p < .45 else 0.0


def tuft_free_bow(progress, maximum):
    p=max(0.0,min(1.0,(progress-.25)/.75))
    return maximum*sin(pi*p)**2


def tuft_fold_relief(t):
    outline=STYLE["upper_swept_tuft_outline"]
    lo=min(p[0] for p in outline);hi=max(p[0] for p in outline)
    u=max(0.0,min(1.0,(t-lo)/(hi-lo)/.45))
    return STYLE["accent_tuft_fold_m"]*u*u*(3-2*u)


def upper_accent_point(side, t, rise, relief, key="upper_lash_rails"):
    anchor = curve(t,2,STYLE["outer_lash_wing_rails"] if key == "outer_lash_wing_rails" else STYLE["upper_lash_rails"])
    x = side*H*(C["eye_center_x"]+C["eye_half_width"]*t)
    y = Y0+H*(C["eye_corner_y"]+C["eye_corner_slope"]*t*STYLE["accent_trajectory_slope_scale"]+rise)
    anchor_y = Y0+H*(C["eye_corner_y"]+C["eye_corner_slope"]*t*STYLE["accent_trajectory_slope_scale"]+anchor)
    thickness = STYLE["accent_band_thickness_m"]
    support = upper_attached_depth(x,anchor_y,thickness)
    if key == "outer_lash_wing_rails":
        # The free outer leaf is an authored sheet, not a skin projection.
        # Its root is joined to the evaluated main band below; preserve a
        # smooth analytic free-tip guide rather than flattening onto the temple.
        support = upper_lid_support(x,anchor_y)+thickness+STYLE["accent_surface_clearance_m"]
        skin = depth(x,anchor_y)+thickness+STYLE["accent_surface_clearance_m"]
        difference = max(-STYLE["accent_outer_root_skin_limit_m"],min(0.0,skin-support))
        support += difference*STYLE["accent_outer_root_skin_blend"]
    slope = max(0.0,anchor-rise)*H*STYLE["accent_sheet_slope"]
    # Keep the physical root on the lid; only open the free sheet into space.
    lower = curve(t,1,STYLE["outer_lash_wing_rails"] if key == "outer_lash_wing_rails" else STYLE["upper_lash_rails"])
    spread = max(0.0,min(1.0,(anchor-rise)/max(1e-9,anchor-lower)))
    # A bowed sheet cross-section, anchored at both authored boundary rails.
    # The crown lifts only interior stations, avoiding a flat extruded shelf.
    if key == "upper_lash_rails":
        y += H*(anchor-lower)*STYLE["accent_cross_arch_ratio"]*sin(pi*spread)
    root_depths = []
    for station, _, root_rise in STYLE["upper_lash_rails"]:
        rx = side*H*(C["eye_center_x"]+C["eye_half_width"]*station)
        ry = Y0+H*(C["eye_corner_y"]+C["eye_corner_slope"]*station*STYLE["accent_trajectory_slope_scale"]+root_rise)
        root_depths.append(upper_attached_depth(rx,ry,thickness))
    opening = min(STYLE["accent_free_edge_envelope_m"],max(0.0,max(root_depths)-support))
    slope += opening*spread*STYLE["accent_free_edge_envelope_blend"] if key != "outer_lash_wing_rails" else 0.0
    if key == "inner_lash_fork_outline":
        slope *= STYLE["accent_inner_fork_sheet_slope_scale"]
    u = max(0.0,min(1.0,(t-STYLE["accent_outer_forward_start_t"])/
        (STYLE["accent_outer_forward_end_t"]-STYLE["accent_outer_forward_start_t"])))
    forward = STYLE["accent_outer_forward_m"]*u*u*(3-2*u)
    if key == "outer_lash_wing_rails":
        forward += STYLE["accent_wing_offset_m"]*u
        wing = STYLE["outer_lash_wing_rails"]
        p = max(0.0,min(1.0,(t-wing[0][0])/(wing[-1][0]-wing[0][0])))
        forward -= STYLE["accent_wing_tip_recede_m"]*p*p*(3.0-2.0*p)
    if key == "upper_swept_tuft_outline":
        outline = STYLE[key]
        rt, rr = min(outline, key=lambda p: p[0])
        tip_t = max(p[0] for p in outline)
        progress = max(0.0,min(1.0,(t-rt)/(tip_t-rt)))
        base = -upper_accent_point(side,rt,rr,0.0)[1]+main_ridge_relief(rt,rr)
        supports = [-upper_accent_point(side,q,curve(q,1,STYLE["upper_lash_rails"]),0.0)[1]
                    for q,_ in outline]
        apex = max(supports)+STYLE["accent_tuft_flare_m"]
        # Two continuous spatial boundary guides define a swept thin leaf.
        # Their separation vanishes at the attached root and pointed tip.
        hits = []
        for a,b in zip(outline,outline[1:]+outline[:1]):
            if abs(a[0]-b[0]) > 1e-10 and min(a[0],b[0]) <= t <= max(a[0],b[0]):
                hits.append(a[1]+(b[1]-a[1])*(t-a[0])/(b[0]-a[0]))
        across = .5 if not hits or max(hits)-min(hits)<1e-9 else max(0.0,min(1.0,(rise-min(hits))/(max(hits)-min(hits))))
        root_heights = [h for q,h in outline if abs(q-rt)<1e-10]
        if len(root_heights)>1:
            root_rise = min(root_heights)+(max(root_heights)-min(root_heights))*across
            base = -upper_accent_point(side,rt,root_rise,0.0)[1]+main_ridge_relief(rt,root_rise)
        upper = progress**STYLE["accent_tuft_upper_depth_power"]
        lower = progress**STYLE["accent_tuft_lower_depth_power"]
        lifted = base+(apex-base)*(lower*(1-across)+upper*across)+tuft_free_bow(progress,STYLE["accent_tuft_free_bow_m"])
        return coord(x,y,lifted-STYLE["accent_tuft_mid_recede_m"]*sin(pi*progress)**2*(1-across)-tuft_rear_scoop(progress,STYLE["accent_tuft_rear_scoop_m"])-STYLE["accent_tuft_root_cover_m"]*(1-progress)**2+relief)
    result = coord(x,y,support+slope+upper_band_bow(t)+forward+relief)
    if key == "inner_lash_fork_outline":
        # A lower corner leaf follows the local lid surface, rather than the
        # depth at the higher main-band anchor. Keep its attached end intact.
        stations = [p[0] for p in STYLE[key]]
        u = max(0.0,min(1.0,(max(stations)-t)/(max(stations)-min(stations))))
        blend = u*u*(3-2*u)*STYLE["accent_inner_fork_surface_follow"]
        local = coord(x,y,upper_attached_depth(x,y,thickness)+relief)
        result = tuple(result[i]*(1-blend)+local[i]*blend for i in range(3))
        top = max(h for _,h in STYLE[key])
        bottom = min(h for _,h in STYLE[key])
        inward = max(0.0,min(1.0,(top-rise)/max(1e-9,top-bottom)))
        result = (result[0],result[1]+STYLE["accent_inner_fork_lower_recede_m"]*inward,result[2])
    if key == "outer_lash_wing_rails":
        rails = STYLE[key]
        u = max(0.0,min(1.0,(t-rails[0][0])/(rails[-1][0]-rails[0][0])))
        blend = u*u*(3-2*u)
        parent = upper_accent_point(side,t,rise,relief)
        result = tuple(parent[i]*(1-blend)+result[i]*blend for i in range(3))
    return result


def build_upper_accents(side):
    for key, name, color, thickness, roll in [
        ("upper_lash_rails", "Upper lash ", INK,
         STYLE["accent_band_thickness_m"], main_band_roll),
        ("inner_lash_fork_outline", "Inner forked lash blade ", INK,
         STYLE["accent_wing_thickness_m"], STYLE["accent_inner_fork_fold_m"]),
        ("upper_swept_tuft_outline", "Upper swept lash tuft ", INK,
         STYLE["accent_wing_thickness_m"], tuft_fold_relief),
        ("outer_lash_wing_rails", "Attached outer lash wing ", tuple(STYLE["accent_outer_tint_rgb"]),
         STYLE["accent_wing_thickness_m"], STYLE["accent_wing_fold_relief_m"]),
    ]:
        def point(t,rise,relief):
            return upper_accent_point(side,t,rise,relief,key)
        obj = accent_mesh(name+str(side),accent_sections(key),point,color,thickness,roll,True)
        if key != "outer_lash_wing_rails":
            obj["lash_gradient"] = True



def lateral_sections():
    controls = [p[0] for p in STYLE["lateral_accent_profile"]]
    stations=set(controls)
    for a,b in zip(controls,controls[1:]):
        n=max(1,int((b-a)/.065+.999999))
        stations.update(a+(b-a)*i/n for i in range(1,n))
    stations=sorted(stations)
    return [(f, -C["outer_liner_width"] * curve(f,1,STYLE["accent_lateral_inner_width_profile"]),
             0.0) for f in stations]


def lateral_position(side, f, offset):
    _, top = eye_contour(side, .95, True)
    _, bottom = eye_contour(side, STYLE["accent_lateral_bottom_t"], False)
    t = curve(f,1,STYLE["accent_lateral_center_profile"])
    x = side*H*(C["eye_center_x"]+C["eye_half_width"]*t+offset)
    return x, top*(1-f)+bottom*f


def lateral_attached_depth(x, y, anchor_x=None):
    """Fit a continuous section anchor; free teeth retain their local curve."""
    baseline = depth(x,y)
    sample_x = x if anchor_x is None else anchor_x
    anchor = depth(sample_x,y)
    support = globals().get("_EVALUATED_LID_SUPPORT")
    if support is not None:
        hit,_,_,_=support.ray_cast(Vector((sample_x,-.5,y)),Vector((0,1,0)),1.0)
        if hit is not None and abs(-hit.y-anchor) <= STYLE["accent_evaluated_support_max_delta_m"]:
            baseline += -hit.y-anchor
    return baseline


def lateral_accent_point(side, f, offset, relief):
    x,y = lateral_position(side,f,offset)
    root_x,_ = lateral_position(side,f,0.0)
    point = coord(x,y,lateral_attached_depth(x,y,root_x)+STYLE["accent_lateral_thickness_m"]
                  +STYLE["accent_surface_clearance_m"]+relief)
    x0,y0 = lateral_position(side,0.0,offset)
    root_x0,_ = lateral_position(side,0.0,0.0)
    old_root = coord(x0,y0,lateral_attached_depth(x0,y0,root_x0)+STYLE["accent_lateral_thickness_m"]
                     +STYLE["accent_surface_clearance_m"]+relief)
    t = STYLE["accent_lateral_join_t"]+offset/C["eye_half_width"]
    rise = curve(t,1,STYLE["outer_lash_wing_rails"])
    joined = upper_accent_point(side,t,rise,relief,"outer_lash_wing_rails")
    u = max(0.0,min(1.0,f/STYLE["accent_lateral_join_span"]))
    weight = 1-u*u*(3-2*u)
    result = tuple(point[i]+(joined[i]-old_root[i])*weight for i in range(3))
    bx,by = lateral_position(side,1.0,0.0)
    bottom_depth = lateral_attached_depth(bx,by)+STYLE["accent_lateral_thickness_m"]+STYLE["accent_surface_clearance_m"]+relief
    forward_depth = (-joined[1])*(1-f)+bottom_depth*f+STYLE["accent_lateral_depth_bow_m"]*sin(pi*f)
    blend = STYLE["accent_lateral_depth_blend"]
    return (result[0],result[1]*(1-blend)-forward_depth*blend,result[2])


def build_lateral_accent(side):
    def point(f,offset,relief):
        return lateral_accent_point(side,f,offset,relief)
    accent_mesh("Outer lateral eye liner "+str(side),lateral_sections(),point,
                tuple(STYLE["accent_outer_tint_rgb"]),STYLE["accent_lateral_thickness_m"],
                STYLE["accent_lateral_roll_m"],True)
    sections = [(i/16,0.0,STYLE["accent_lateral_branch_width_H"]*sin(pi*i/16)**.8)
                for i in range(17)]
    def branch(f,width,relief):
        along = STYLE["accent_lateral_branch_start"]+STYLE["accent_lateral_branch_span"]*f
        separation = STYLE["accent_lateral_branch_separation_H"]*sin(pi*f)**.85
        return lateral_accent_point(side,along,separation+width,relief+.00012)
    accent_mesh("Outer companion lash "+str(side),sections,branch,
                tuple(STYLE["accent_outer_tint_rgb"]),STYLE["accent_lateral_thickness_m"],
                STYLE["accent_lateral_roll_m"]*.5,True)


def build_lower_tuft(side):
    # One small independent tapered tuft rooted at the outer lower lid.
    center = STYLE["accent_lower_tip_t"]
    width = STYLE["accent_lower_tip_width_t"]
    outline = [(center-width,0.0),(center+.02,-STYLE["accent_lower_tip_drop_H"]),
               (center+width,.001),(center,.003)]
    def point(t,rise,relief):
        x,y = eye_contour(side,t,False)
        y += H*rise
        return coord(x,y,lash_depth(x,y)+relief)
    accent_mesh("Lower outer lash tuft "+str(side),outline_sections(outline),point,
                (.12,.08,.075),STYLE["accent_lower_tip_thickness_m"],.00008)


def fold_width(f):
    return (STYLE["lid_fold_width_H"] * max(0, sin(pi*f)) ** .65
            * (1 + STYLE["lid_fold_head_fullness"] * (1-2*f)))


def build_lid_fold(side):
    sections = [(i/16, 0.0, fold_width(i/16)) for i in range(17)]
    def point(f, height, relief):
        t = -.76 + 1.52*f
        x = side * H * (C["eye_center_x"] + C["eye_half_width"]*t)
        y = Y0 + H * (curve(f, 1, STYLE["lid_fold_height_profile"]) + height)
        return coord(x, y, depth(x, y) + STYLE["accent_fold_forward_m"] + relief)
    accent_mesh("Tapered upper eyelid fold " + str(side), sections, point,
                (0.76, 0.47, 0.45), STYLE["accent_fold_thickness_m"],
                STYLE["accent_fold_roll_m"])


def triangle_samples(a, b, c, divisions=4):
    """Include facet interiors, where a curved obstacle can pierce a chord."""
    for i in range(divisions+1):
        for j in range(divisions+1-i):
            u, v = i/divisions, j/divisions
            yield tuple(a[k]*(1-u-v)+b[k]*u+c[k]*v for k in range(3))


def clearance_shift(point, support_y, margin):
    if support_y is None:
        return 0.0
    return min(0.0, support_y-margin-point[1])


def main_root_shift_weight(rise, lo, hi):
    width = max(0.0, hi-lo)
    u = max(0.0, min(1.0, (rise-lo)/max(1e-9,width)))
    taper = min(1.0, width/.006)
    return (1-taper)+taper*u*u*(3-2*u)


def fold_support_shift_weight(rise, lo, hi):
    """Preserve the attached upper facet without dragging the free margin forward."""
    width = max(0.0, hi-lo)
    ridge = STYLE['accent_fold_ridge_fraction']
    crest = ridge + STYLE['accent_cross_arch_ratio']*sin(pi*ridge)
    u = max(0.0,min(1.0,(rise-lo)/max(1e-9,width*crest)))
    free = STYLE['accent_free_margin_support_fraction']
    weight = free+(1-free)*u*u*(3-2*u)
    taper = min(1.0,width/.006)
    return (1-taper)+taper*weight


def extend_root_shifts(rows):
    valid = [row for row in rows if row[1] is not None]
    if not valid:
        return [[x, 0.0] for x, _ in rows]
    return [[x, curve(x,1,valid) if value is None else value] for x,value in rows]


def fit_shared_upper_attachment(tree):
    """Move connected thin pieces together when the evaluated lid differs from its cage."""
    profiles, free_profiles = {}, {}
    for side in (-1,1):
        main = next(o for o in PARTS if o.name == "Upper lash "+str(side))
        columns = {}
        for vertex in main.data.vertices:
            columns.setdefault(round(abs(vertex.co.x),8),[]).append(vertex)
        rows, free_rows = [], []
        for x, vertices in sorted(columns.items()):
            z = max(v.co.z for v in vertices)
            back = max(v.co.y for v in vertices if abs(v.co.z-z)<1e-7)
            hit,_,_,_=tree.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)),1.0)
            shift = None if hit is None else min(0.0,hit.y-STYLE['accent_evaluated_clearance_m']-back)
            assert shift is None or abs(shift)<.012,'Shared root requires a new structural design'
            rows.append([x,shift])
            low = min(v.co.z for v in vertices)
            free_back = max(v.co.y for v in vertices if abs(v.co.z-low)<1e-7)
            free_hit,_,_,_=tree.ray_cast(Vector((x,-.5,low)),Vector((0,1,0)),1.0)
            free_shift = None if free_hit is None else free_hit.y-STYLE['accent_evaluated_clearance_m']-free_back
            # A distant eye/interior hit is not a local eyelid support.
            if free_shift is not None and abs(free_shift)>STYLE['accent_free_support_reach_m']:
                free_shift = None
            assert free_shift is None or abs(free_shift)<.012,'Free margin requires a new structural design'
            free_rows.append([x,free_shift])
        profiles[side] = extend_root_shifts(rows)
        free_profiles[side] = extend_root_shifts(free_rows)
    for obj in PARTS:
        upper = any(obj.name.startswith(n) for n in ['Upper lash ','Inner forked lash blade ','Upper swept lash tuft ','Attached outer lash wing '])
        lateral = obj.name.startswith('Outer lateral eye liner ')
        if not upper and not lateral:
            continue
        side = -1 if obj.name.endswith('-1') else 1
        high = max(v.co.z for v in obj.data.vertices)
        low = min(v.co.z for v in obj.data.vertices)
        for vertex in obj.data.vertices:
            weight, free_weight = 1.0, 0.0
            if upper:
                t = (abs(vertex.co.x)/H-C['eye_center_x'])/C['eye_half_width']
                rise = (vertex.co.z-Y0)/H-C['eye_corner_y']-C['eye_corner_slope']*t*STYLE['accent_trajectory_slope_scale']
                lo = curve(t,1,STYLE['upper_lash_rails'])
                hi = curve(t,2,STYLE['upper_lash_rails'])
                # Root collision correction is not a rigid-body translation:
                # the free margin is already forward of the supported skin.
                u = max(0.0,min(1.0,(rise-lo)/max(1e-9,hi-lo)))
                weight = u*u*(3-2*u)
                if obj.name.startswith("Upper lash "):
                    weight = main_root_shift_weight(rise,lo,hi)
                # Preserve the authored fold cross-section when the evaluated
                # lid moves its root. Root-only correction flattened the band.
                preserve = STYLE['accent_support_section_preservation']
                weight = preserve*fold_support_shift_weight(rise,lo,hi)+(1-preserve)*weight
                free_weight = STYLE['accent_free_surface_follow']*(1-weight)/max(1e-9,1-STYLE['accent_free_margin_support_fraction'])
            if obj.name.startswith('Inner forked lash blade '):
                # The fork has its own evaluated support; the main band's free
                # rail lies at another height and must not drag this leaf.
                free_weight = 0.0
            if obj.name.startswith('Upper swept lash tuft '):
                # Its two spatial boundaries already define an independent leaf.
                # Do not deform it using a different-height main free rail.
                free_weight = 0.0
            if obj.name.startswith('Attached outer lash wing '):
                # Its root attaches to the main free margin, but its tip has
                # an independent guide. Preserve root displacement and fade out.
                wing_rails = STYLE['outer_lash_wing_rails']
                progress = max(0.0, min(1.0, (t-wing_rails[0][0]) /
                    max(1e-9, wing_rails[-1][0]-wing_rails[0][0])))
                free_weight *= (1.0-progress)**2
            if lateral:
                u = max(0.0,min(1.0,(high-vertex.co.z)/max(1e-9,(high-low)*STYLE['accent_lateral_join_span'])))
                weight = STYLE['accent_free_margin_support_fraction'] * (1-u*u*(3-2*u))  # shared free-rail correction fades down the side
                free_weight = STYLE['accent_free_surface_follow']*(1-u*u*(3-2*u))
            vertex.co.y += curve(abs(vertex.co.x),1,profiles[side])*weight+curve(abs(vertex.co.x),1,free_profiles[side])*free_weight
        obj.data.update()
    return {str(s):max(abs(row[1]) for row in rows) for s,rows in profiles.items()}


def rebuild_accents_on_evaluated_support(tree):
    """Re-evaluate original parametric guides on the final generated surface."""
    global _EVALUATED_LID_SUPPORT
    _EVALUATED_LID_SUPPORT = tree
    original = {o.name:o for o in PARTS}
    start = len(PARTS)
    try:
        for side in (-1,1):
            build_upper_accents(side)
            build_lateral_accent(side)
        created = list(PARTS[start:])
        for new in created:
            name = new.name.rsplit('.',1)[0]
            assert name in original, 'Missing original accent during support rebuild'
            old = original[name]
            assert len(old.data.vertices)==len(new.data.vertices), 'Support rebuild changed topology'
            old.data = new.data.copy()
            old['accent_positive_volume_m3'] = new['accent_positive_volume_m3']
            PARTS.remove(new)
            bpy.data.objects.remove(new,do_unlink=True)
    finally:
        _EVALUATED_LID_SUPPORT = None


def final_free_rail_delta(lower_y, upper_y, height, slope, limit):
    """Blender negative Y is forward. Prevent the final free edge folding behind its root."""
    delta = min(0.0, upper_y-max(0.0,height)*slope-lower_y)
    assert abs(delta) <= limit, 'Final hinge exceeds bounded authoring envelope'
    return delta


def restore_final_main_hinge():
    """Correct only owned main sheets after attachment; transport attached leaf roots."""
    from mathutils.bvhtree import BVHTree
    result = {}
    for side in (-1,1):
        main = next(o for o in PARTS if o.name == 'Upper lash '+str(side))
        before = BVHTree.FromPolygons([v.co.copy() for v in main.data.vertices],
                                     [tuple(p.vertices) for p in main.data.polygons])
        columns = {}
        for v in main.data.vertices:
            columns.setdefault(round(abs(v.co.x),8),[]).append(v)
        rows = []
        for x,col in sorted(columns.items()):
            low=min(v.co.z for v in col); high=max(v.co.z for v in col)
            ly=min(v.co.y for v in col if abs(v.co.z-low)<1e-7)
            hy=min(v.co.y for v in col if abs(v.co.z-high)<1e-7)
            shift=final_free_rail_delta(ly,hy,high-low,
                STYLE['accent_final_min_forward_slope'],STYLE['accent_final_max_hinge_shift_m'])
            rows.append([x,shift,low,high])
        def displacement(point):
            x=abs(point.x); low=curve(x,2,rows); high=curve(x,3,rows)
            q=max(0.0,min(1.0,(point.z-low)/max(1e-9,high-low)))
            return curve(x,1,rows)*(1-q)
        for v in main.data.vertices:
            v.co.y+=displacement(v.co)
        main.data.update()
        for prefix in ['Attached outer lash wing ','Upper swept lash tuft ','Outer lateral eye liner ']:
            obj=next(o for o in PARTS if o.name==prefix+str(side))
            lateral=prefix.startswith('Outer lateral')
            root=obj.data.vertices[0].co.copy()
            if lateral:
                root=sum((v.co for v in list(obj.data.vertices)[:6]),Vector())/6
            near=before.find_nearest(root)[0]
            assert near is not None
            shift=displacement(near)
            x0=abs(root.x); x1=max(abs(v.co.x) for v in obj.data.vertices)
            z0=max(v.co.z for v in obj.data.vertices); z1=min(v.co.z for v in obj.data.vertices)
            for i,v in enumerate(obj.data.vertices):
                if lateral:
                    progress=max(0.0,min(1.0,(z0-v.co.z)/max(1e-9,(z0-z1)*STYLE['accent_lateral_join_span'])))
                    if i<6:progress=0.0
                else:
                    progress=max(0.0,min(1.0,(abs(v.co.x)-x0)/max(1e-9,x1-x0)))
                v.co.y+=shift*(1-progress)**2
            obj.data.update()
        result[str(side)]={'maximum_free_rail_shift_m':max(abs(r[1]) for r in rows),
                           'minimum_authored_slope':STYLE['accent_final_min_forward_slope']}
    return result


def fitted_crown_relief(width_m, height_m, nominal_relief, roll_ratio):
    """The final fold fades with local strip width just like its authored cross-section."""
    return nominal_relief*roll_ratio*min(1.0,max(0.0,width_m)/(0.014*height_m))


def fitted_crown_depth(lower_y, upper_y, fraction, relief):
    return lower_y*(1-fraction)+upper_y*fraction-relief


def restore_fitted_main_fold():
    """Bound crown against final attached rails, not their pre-projection positions."""
    from mathutils.bvhtree import BVHTree
    result = {}
    for side in (-1, 1):
        main = next(o for o in PARTS if o.name == 'Upper lash '+str(side))
        tuft = next(o for o in PARTS if o.name == 'Upper swept lash tuft '+str(side))
        faces = [tuple(p.vertices) for p in main.data.polygons]
        before = BVHTree.FromPolygons([v.co.copy() for v in main.data.vertices], faces)
        attached_before = before.find_nearest(tuft.data.vertices[0].co)[0]
        rows = {}
        for v in main.data.vertices:
            rows.setdefault(round(abs(v.co.x),8), []).append(v)
        changes = []
        for x, column in sorted(rows.items()):
            if len(column)==1:
                continue
            assert len(column)==6, 'Unexpected main ribbon station topology'
            levels = sorted(set(round(v.co.z,7) for v in column))
            assert len(levels)==3, 'Main station must contain two boundary rails and crown'
            groups = [[v for v in column if abs(v.co.z-z)<1e-7] for z in levels]
            assert all(len(g)==2 for g in groups)
            low, crest, high = [sorted(g,key=lambda v:v.co.y) for g in groups]
            q=STYLE['accent_final_crown_height_fraction']
            assert .1 <= q <= .9, 'Crown must remain inside the two protected rails'
            for v in crest: v.co.z=low[0].co.z+(high[0].co.z-low[0].co.z)*q
            t=(x/H-C['eye_center_x'])/C['eye_half_width']
            ratio=main_band_roll(t)/STYLE['accent_band_roll_m']
            relief=fitted_crown_relief(high[0].co.z-low[0].co.z,H,STYLE['accent_final_crown_relief_m'],ratio)
            target=fitted_crown_depth(low[0].co.y,high[0].co.y,q,relief)
            offset=target-crest[0].co.y
            # Matched return pair retains its exact physical sheet thickness.
            for v in crest:
                v.co.y+=offset
            changes.append(offset)
        main.data.update()
        after=BVHTree.FromPolygons([v.co.copy() for v in main.data.vertices],faces)
        attached_after=after.find_nearest(attached_before)[0]
        delta=attached_after-attached_before
        x0=abs(tuft.data.vertices[0].co.x)
        x1=max(abs(v.co.x) for v in tuft.data.vertices)
        for v in tuft.data.vertices:
            u=max(0.0,min(1.0,(abs(v.co.x)-x0)/max(1e-9,x1-x0)))
            v.co+=delta*(1-u)**2
        tuft.data.update()
        result[str(side)]={'maximum_crown_change_m':max(map(abs,changes),default=0),
                           'tuft_root_shift_m':delta.length,'stations':len(changes)}
    return result


def fit_evaluated_accent_roots(shell):
    """Fit accent rails outside generated skin, lids AND eye obstacles."""
    from mathutils.bvhtree import BVHTree
    bpy.context.view_layer.update()
    graph = bpy.context.evaluated_depsgraph_get()
    evaluated = shell.evaluated_get(graph)
    data = evaluated.to_mesh()
    try:
        vertices = [v.co.copy() for v in data.vertices]
        faces = [tuple(p.vertices) for p in data.polygons]
        obstacle_trees = [(shell.name, BVHTree.FromPolygons(vertices,faces))]
        for obj in PARTS:
            if 'skin lid' not in obj.name and not obj.get('ocular_surface'):
                continue
            obstacle_trees.append((obj.name, BVHTree.FromPolygons([v.co.copy() for v in obj.data.vertices],[tuple(p.vertices) for p in obj.data.polygons])))
            offset = len(vertices)
            vertices.extend(v.co.copy() for v in obj.data.vertices)
            faces.extend(tuple(offset+i for i in p.vertices) for p in obj.data.polygons)
        tree = BVHTree.FromPolygons(vertices,faces)
        support_sources = {}
        for obj in PARTS:
            if not obj.name.startswith("Upper lash "):
                continue
            columns = {}
            for vertex in obj.data.vertices:
                columns.setdefault(round(abs(vertex.co.x),8),[]).append(vertex)
            for x, column in columns.items():
                z = max(v.co.z for v in column)
                back = max(v.co.y for v in column if abs(v.co.z-z)<1e-7)
                for name, obstacle in obstacle_trees:
                    hit,_,_,_=obstacle.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)),1.0)
                    if hit is not None:
                        needed=max(0.0,back-hit.y+STYLE['accent_evaluated_clearance_m'])
                        support_sources[name]=max(support_sources.get(name,0.0),needed)
        rebuild_accents_on_evaluated_support(tree)
        shared = fit_shared_upper_attachment(tree)
        final_hinge = restore_final_main_hinge()
        fitted_fold = restore_fitted_main_fold()
        curved_main = refine_main_cross_sections(PARTS)
        lower_flank = shape_lower_flanks(PARTS)
        tuft_seating = seat_tuft_roots_on_parent(PARTS)
        wing_seating = seat_outer_wings(PARTS, STYLE, obstacle_trees[0][1])
        lateral_join = reattach_lateral_to_final_wing()
        lateral_curve = regularize_lateral_sections(PARTS, lateral_sections(), STYLE, obstacle_trees[0][1])
        from side_leaf import build_side_leaves
        build_side_leaves(PARTS, lateral_sections(), STYLE)
        report = {"short_leaf_parent_seating":tuft_seating,"lower_main_flank":lower_flank}
        for obj in PARTS:
            if not obj.get('accent_front_faces'):
                continue
            groups = {}
            for v in obj.data.vertices:
                groups.setdefault((round(abs(v.co.x),8),round(v.co.z,8)),[]).append(v)
            if obj.name.startswith(('Outer lateral eye liner ', 'Outer companion lash ')):
                if obj.name.startswith('Outer lateral eye liner '):
                    counts=[1 if hi-lo<1e-9 else 6 for _,lo,hi in lateral_sections()]
                else:
                    counts=[1 if i in (0,16) else 6 for i in range(17)]
                groups={};offset=0
                for count in counts:
                    group=list(obj.data.vertices)[offset:offset+count]
                    groups[offset]=group;offset+=count
                assert offset==len(obj.data.vertices), 'Unexpected rigid lateral sections'
            moves=[]
            for group in groups.values():
                shift=0.0
                for vertex in group:
                    x,y,z=vertex.co
                    hit,_,_,_=tree.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)),1.0)
                    if hit is not None:
                        shift=min(shift,hit.y-STYLE['accent_evaluated_clearance_m']-y)
                assert abs(shift)<.012,'Accent root requires a new structural design'
                for v in group:v.co.y+=shift
                moves.append(abs(shift))
            # Curved iris rims can cross a long facet between clear vertices.
            # Correct shared front/return columns together, then recheck interiors.
            vertex_groups = {v.index: group for group in groups.values() for v in group}
            maximum_facet_shift = 0.0
            sampled_clearance = float('inf')
            for iteration in range(8):
                corrections = {}
                sampled_clearance = float('inf')
                for polygon in obj.data.polygons:
                    ids = list(polygon.vertices)
                    for j in range(1,len(ids)-1):
                        tri = (ids[0],ids[j],ids[j+1])
                        points = [obj.data.vertices[i].co for i in tri]
                        needed = 0.0
                        for sample in triangle_samples(*points):
                            x,y,z = sample
                            hit,_,_,_ = tree.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)),1.0)
                            if hit is None:
                                continue
                            sampled_clearance = min(sampled_clearance,hit.y-y)
                            needed = min(needed,clearance_shift(sample,hit.y,
                                         STYLE['accent_evaluated_clearance_m']))
                        if needed < -1e-7:
                            for i in tri:
                                key = vertex_groups[i][0].index
                                corrections[key] = min(corrections.get(key,0.0),needed)
                if not corrections:
                    break
                assert iteration < 7, 'Accent facets still intersect generated obstacles'
                for key,shift in corrections.items():
                    assert abs(shift)<.012, 'Unexpected accent collision displacement'
                    maximum_facet_shift = max(maximum_facet_shift,abs(shift))
                    for vertex in vertex_groups[key]:
                        vertex.co.y += shift
            if obj.name.startswith(('Outer lateral eye liner ', 'Outer companion lash ')):
                assert maximum_facet_shift < .001, 'Side ribbon requires structural correction, not a large collision spike'
            assert sampled_clearance >= STYLE['accent_evaluated_clearance_m']-2e-7
            obj['minimum_obstacle_clearance_m'] = sampled_clearance
            obj['maximum_facet_correction_m'] = maximum_facet_shift
            obj.data.update()
            assert all(p.area>1e-12 for p in obj.data.polygons),'Degenerate fitted accent'
            report[obj.name]={'curved_main':curved_main,'outer_wing_seating':wing_seating,'lateral_final_curve':lateral_curve,'lateral_wing_join':lateral_join,'final_main_hinge':final_hinge,'fitted_main_fold':fitted_fold,'root_obstacle_corrections_by_source_m':support_sources, 'shared_upper_attachment_maximum_m':shared, 'sampled_roots':len(moves),'maximum_forward_correction_m':max(moves,default=0),
                              'minimum_requested_skin_clearance_m':STYLE['accent_evaluated_clearance_m'],
                              'includes_generated_ocular_obstacles':True,
                              'minimum_sampled_obstacle_clearance_m':sampled_clearance,
                              'maximum_facet_correction_m':maximum_facet_shift}
        return report
    finally:
        evaluated.to_mesh_clear()


def reattach_lateral_to_final_wing():
    """Keep the lower side band under its wing, without filling the upper fork gap."""
    from mathutils.bvhtree import BVHTree
    report = {}
    for side in (-1,1):
        wing = next(o for o in PARTS if o.name == 'Attached outer lash wing '+str(side))
        lateral = next(o for o in PARTS if o.name == 'Outer lateral eye liner '+str(side))
        tree = BVHTree.FromPolygons([v.co.copy() for v in wing.data.vertices], [tuple(p.vertices) for p in wing.data.polygons])
        roots = list(lateral.data.vertices)[:6]
        center = sum((v.co for v in roots),Vector())/6
        target = tree.find_nearest(center)[0]
        assert target is not None, 'Missing lateral wing parent'
        delta = target-center
        assert delta.length < .004, 'Lateral parent transport exceeds design bound'
        top = max(v.co.z for v in lateral.data.vertices)
        bottom = min(v.co.z for v in lateral.data.vertices)
        for v in lateral.data.vertices:
            u = max(0.0,min(1.0,(top-v.co.z)/max(1e-9,(top-bottom)*STYLE['accent_lateral_join_span'])))
            if v.index < 6: u=0.0
            v.co += delta*(1-u*u*(3-2*u))
        lateral.data.update()
        report[str(side)] = {'parent':'outer_wing','translation_m':delta.length}
    return report


def measure_accent_connections():
    from mathutils.bvhtree import BVHTree
    result = {}
    for side in (-1,1):
        upper = next(o for o in PARTS if o.name == "Upper lash "+str(side))
        lateral = next(o for o in PARTS if o.name == "Outer lateral eye liner "+str(side))
        tree = BVHTree.FromPolygons([v.co.copy() for v in upper.data.vertices],
                    [tuple(p.vertices) for p in upper.data.polygons])
        wing = next(o for o in PARTS if o.name == "Attached outer lash wing "+str(side))
        wing_tree = BVHTree.FromPolygons([v.co.copy() for v in wing.data.vertices], [tuple(p.vertices) for p in wing.data.polygons])
        distances = [wing_tree.find_nearest(v.co)[3] for v in list(lateral.data.vertices)[:6]]
        assert all(d is not None for d in distances)
        wing_distance = tree.find_nearest(wing.data.vertices[0].co)[3]
        assert wing_distance is not None and wing_distance < .0008, "Secondary wing root disconnected"
        tuft = next(o for o in PARTS if o.name == "Upper swept lash tuft "+str(side))
        tuft_distance = tree.find_nearest(tuft.data.vertices[0].co)[3]
        assert tuft_distance is not None and tuft_distance < .0008, f"Short tuft root disconnected: {tuft_distance}"
        maximum = max(distances)
        assert maximum < .0008, "Wing/lateral three-dimensional root disconnected"
        result[str(side)] = {"lateral_parent":"outer_wing","sampled_root_vertices":6,"short_tuft_root_distance_m":tuft_distance,"secondary_wing_root_distance_m":wing_distance,"maximum_root_surface_distance_m":maximum,
                            "maximum_allowed_distance_m":.0008}
    return result
