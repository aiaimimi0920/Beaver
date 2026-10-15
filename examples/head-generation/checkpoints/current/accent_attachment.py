"""Shared lash attachment, hinge and crown fitting on evaluated surfaces."""
from face import *


def main_band_roll(t):
    rails = STYLE["upper_lash_rails"]
    phase = max(0.0,min(1.0,(t-rails[0][0])/(rails[-1][0]-rails[0][0])))
    low = STYLE["accent_band_end_roll_factor"]
    high = STYLE["accent_band_center_roll_factor"]
    return STYLE["accent_band_roll_m"]*(low+(high-low)*sin(pi*phase))


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
