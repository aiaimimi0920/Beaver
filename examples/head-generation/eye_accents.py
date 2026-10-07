"""Original thin folded eye accents, independent of the protected eye surfaces."""
from face import *


def outline_sections(outline):
    """Cross-sections of an x-monotone semantic outline, including pointed ends."""
    result = []
    edges = list(zip(outline, outline[1:] + outline[:1]))
    controls = sorted({p[0] for p in outline})
    stations = set(controls)
    for a,b in zip(controls,controls[1:]):
        count = max(1,int((b-a)/STYLE["accent_max_station_gap"])+1)
        stations.update(a+(b-a)*i/count for i in range(1,count))
    for x in sorted(stations):
        hits = []
        for a, b in edges:
            if abs(a[0] - b[0]) < 1e-10:
                if abs(x - a[0]) < 1e-10:
                    hits.extend([a[1], b[1]])
            elif min(a[0], b[0]) - 1e-10 <= x <= max(a[0], b[0]) + 1e-10:
                f = (x - a[0]) / (b[0] - a[0])
                hits.append(a[1] + f * (b[1] - a[1]))
        assert hits
        result.append((x, min(hits), max(hits)))
    return result


def signed_volume(vertices, faces):
    origin = vertices[0]
    volume = 0.0
    for face in faces:
        for j in range(1, len(face) - 1):
            a, b, c = [tuple(vertices[i][k] - origin[k] for k in range(3))
                       for i in (face[0], face[j], face[j + 1])]
            volume += (a[0]*(b[1]*c[2]-b[2]*c[1])
                       - a[1]*(b[0]*c[2]-b[2]*c[0])
                       + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6
    return volume


def closed_ribbon_mesh(sections, point, thickness, roll):
    """Matched six-rail thin return; pointed ends use bounded-valence caps, not tiny loops."""
    vertices, rings, faces, zones = [], [], [], []
    for t, lower, upper in sections:
        if upper - lower < 1e-9:
            rings.append([len(vertices)])
            vertices.append(point(t, lower, 0.0))
            continue
        ridge = lower + (upper - lower) * .42
        relief = roll * min(1.0, (upper - lower) / .014)
        section = [(lower, -thickness), (lower, 0.0), (ridge, relief),
                   (upper, 0.0), (upper, -thickness), (ridge, -thickness)]
        rings.append(list(range(len(vertices), len(vertices) + len(section))))
        vertices.extend(point(t, height, offset) for height, offset in section)
    for first, second in zip(rings, rings[1:]):
        assert len(first) + len(second) > 2, "Adjacent collapsed stations"
        for lane in range(6):
            nxt = (lane + 1) % 6
            if len(first) == 1:
                face = (first[0], second[nxt], second[lane])
            elif len(second) == 1:
                face = (first[lane], first[nxt], second[0])
            else:
                face = (first[lane], first[nxt], second[nxt], second[lane])
            faces.append(face)
            zones.append(lane)
    for ring, reverse in [(rings[0], True), (rings[-1], False)]:
        if len(ring) > 1:
            faces.append(tuple(reversed(ring)) if reverse else tuple(ring))
            zones.append(6)
    volume = signed_volume(vertices, faces)
    assert abs(volume) > 1e-14, "Collapsed accent volume"
    if volume < 0:
        faces = [(face[0], *reversed(face[1:])) for face in faces]
    return vertices, faces, zones


def accent_mesh(name, sections, point, color, thickness, roll):
    vertices, faces, zones = closed_ribbon_mesh(sections, point, thickness, roll)
    obj = mesh(name, vertices, faces, color)
    attribute = obj.data.attributes.new("AccentNormalZone", "INT", "FACE")
    front = []
    for polygon, zone in zip(obj.data.polygons, zones):
        attribute.data[polygon.index].value = zone
        polygon.use_smooth = zone in (1, 2)
        if zone in (1, 2):
            front.append(polygon.index)
    obj["accent_front_faces"] = front
    obj["accent_positive_volume_m3"] = abs(signed_volume(vertices, faces))
    assert all(p.area > 1e-12 for p in obj.data.polygons), "Degenerate accent face"
    return obj


def upper_band_bow(t):
    outline = STYLE["upper_lash_outline"]
    lo, hi = min(p[0] for p in outline), max(p[0] for p in outline)
    phase = max(0.0, min(1.0, (t - lo) / (hi - lo)))
    return STYLE["accent_band_bow_m"] * sin(pi * phase)


def upper_lid_support(x, y):
    """Existing skin-lid four-lane surface; no changes to the accepted lid."""
    side = 1 if x >= 0 else -1
    # Free tips continue the local lid support, not the distant cheek surface.
    limit = H*(C["eye_center_x"]+C["eye_half_width"]*STYLE["accent_corner_support_t"])
    x = side*min(abs(x),limit)
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
    return upper_lid_support(x,y)+thickness+STYLE["accent_surface_clearance_m"]


def build_upper_accents(side):
    for key, name, color, offset, thickness, roll in [
        ("upper_lash_outline", "Upper lash ", INK, 0.0,
         STYLE["accent_band_thickness_m"], STYLE["accent_band_roll_m"]),
        ("upper_swept_tuft_outline", "Upper swept lash tuft ", INK, .00020,
         STYLE["accent_wing_thickness_m"], .00006),
        ("outer_lash_wing_outline", "Attached outer lash wing ", (0.23, 0.065, 0.14),
         STYLE["accent_wing_offset_m"], STYLE["accent_wing_thickness_m"],
         STYLE["accent_band_roll_m"] * .45),
    ]:
        cross_sections = outline_sections(STYLE[key])
        def point(t, rise, relief):
            lower = curve(t, 1, cross_sections)
            upper = curve(t, 2, cross_sections)
            free_edge = max(0.0,min(1.0,(rise-lower)/max(1e-9,upper-lower)))
            projection = STYLE["accent_lip_projection_m"]*free_edge
            x = side * H * (C["eye_center_x"] + C["eye_half_width"] * t)
            y = Y0 + H * (C["eye_corner_y"] + C["eye_corner_slope"] * t + rise)
            return coord(x, y, upper_attached_depth(x, y, thickness) + upper_band_bow(t) + offset + relief + projection)
        accent_mesh(name + str(side), outline_sections(STYLE[key]), point,
                    color, thickness, roll)


def lateral_sections():
    controls = [p[0] for p in STYLE["lateral_accent_profile"]]
    stations = sorted(controls + [(a+b)/2 for a,b in zip(controls, controls[1:])
                                  if b-a > .15])
    return [(f, -C["outer_liner_width"] * max(0.0, sin(pi*f)) ** .65,
             curve(f, 1, STYLE["lateral_accent_profile"])) for f in stations]


def lateral_position(side, f, offset):
    _, top = eye_contour(side, .95, True)
    _, bottom = eye_contour(side, .55, False)
    t = .95 - .23*f + .06*sin(pi*f)
    x = side*H*(C["eye_center_x"]+C["eye_half_width"]*t+offset)
    return x, top*(1-f)+bottom*f


def build_lateral_accent(side):
    def point(f, offset, relief):
        x,y = lateral_position(side,f,offset)
        root_x,_ = lateral_position(side,f,0.0)
        return coord(x,y,depth(root_x,y)+STYLE["accent_lateral_thickness_m"]
                     +STYLE["accent_surface_clearance_m"]+relief)
    accent_mesh("Outer lateral eye liner "+str(side),lateral_sections(),point,
                (.23,.065,.14),STYLE["accent_lateral_thickness_m"],
                STYLE["accent_lateral_roll_m"])
    # A separate swept companion blade, leaving an intentional open slit.
    sections = [(i/16,0.0,STYLE["accent_lateral_branch_width_H"]*sin(pi*i/16)**.8)
                for i in range(17)]
    def branch(f, width, relief):
        along = .05+.89*f
        separation = STYLE["accent_lateral_branch_separation_H"]*sin(pi*f)**.85
        x,y = lateral_position(side,along,separation+width)
        root_x,_ = lateral_position(side,along,0.0)
        return coord(x,y,depth(root_x,y)+STYLE["accent_lateral_thickness_m"]
                     +STYLE["accent_surface_clearance_m"]+.00012+relief)
    accent_mesh("Outer companion lash "+str(side),sections,branch,
                (.23,.065,.14),STYLE["accent_lateral_thickness_m"],
                STYLE["accent_lateral_roll_m"]*.5)


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


def fit_evaluated_accent_roots(shell):
    """Fit original accent rails to this generated skin, never to reference data."""
    from mathutils.bvhtree import BVHTree
    bpy.context.view_layer.update()
    graph = bpy.context.evaluated_depsgraph_get()
    evaluated = shell.evaluated_get(graph)
    data = evaluated.to_mesh()
    try:
        vertices = [v.co.copy() for v in data.vertices]
        faces = [tuple(p.vertices) for p in data.polygons]
        for obj in PARTS:
            if 'skin lid' not in obj.name:
                continue
            offset = len(vertices)
            vertices.extend(v.co.copy() for v in obj.data.vertices)
            faces.extend(tuple(offset+i for i in p.vertices) for p in obj.data.polygons)
        tree = BVHTree.FromPolygons(vertices,faces)
        report = {}
        for obj in PARTS:
            if not obj.get('accent_front_faces'):
                continue
            groups = {}
            for v in obj.data.vertices:
                groups.setdefault((round(abs(v.co.x),8),round(v.co.z,8)),[]).append(v)
            moves=[]
            for (x,z),group in groups.items():
                hit,_,_,_=tree.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)),1.0)
                if hit is None:
                    continue
                # A shared shift keeps each return/front pair's thickness intact.
                back=max(v.co.y for v in group)
                target=hit.y-STYLE['accent_evaluated_clearance_m']
                shift=min(0.0,target-back)
                assert abs(shift)<.012,'Accent root requires a new structural design'
                for v in group:v.co.y+=shift
                moves.append(abs(shift))
            obj.data.update()
            assert all(p.area>1e-12 for p in obj.data.polygons),'Degenerate fitted accent'
            report[obj.name]={'sampled_roots':len(moves),'maximum_forward_correction_m':max(moves,default=0),
                              'minimum_requested_skin_clearance_m':STYLE['accent_evaluated_clearance_m']}
        return report
    finally:
        evaluated.to_mesh_clear()
