"""Shallow open-backed pinna membrane sharing evaluated face-root positions."""

from face import *


def build_ears():
    # Deferred until the facial subdivision is evaluated, so root positions match.
    return None


def membrane_offset(t, u, config, root_depth):
    envelope = sin(pi * t) ** (1 - (1-config["ear_lower_arc_power"])*(1-t)**3)
    envelope *= 1 + config["ear_upper_fullness"] * (2 * t - 1)
    envelope *= (
        1
        - config["ear_lower_taper"] * (1 - t) ** 2
        - config["ear_upper_trim"] * max(0, (t - 0.7) / 0.3) ** 2
    )
    upper_return = max(0, min(1, (t - 0.55) / 0.45))
    upper_return = upper_return * upper_return * (3 - 2 * upper_return)
    outward = envelope * config["ear_outward_span"] * curve(u, 1, config["ear_outward_profile"])
    ease = min(1.0, max(0.0, t / config["ear_lower_sweep_ease_end"]))
    ease = ease * ease * (3.0 - 2.0 * ease)
    outer_depth = (
        config["ear_root_depth_base"]
        + config["ear_root_depth_slope"] * t
        - config["ear_posterior_span"] * envelope
        - config["ear_lower_posterior_sweep"] * sin(pi*t)**0.4 * (1-t)**3 * ease
        + config["ear_upper_depth_return"] * upper_return
    )
    outer_depth = min(outer_depth, root_depth - config["ear_min_posterior_clearance"] * sin(pi * t))
    sweep = curve(u, 1, config["ear_depth_profile"])
    posterior = (root_depth - outer_depth) * sweep
    return outward, posterior


def ear_arc_height(t, low, high):
    return (
        low + (high - low) * (1 - cos(pi * t)) / 2 + H * C["ear_upper_arc_bias"] * t * sin(pi * t)
        + (high-low) * C["ear_lower_height_linear_blend"] * max(0,1-2*t)**2 * (t-(1-cos(pi*t))/2)
    )


def project_ear_uv(vertices):
    depths = [p[1] for p in vertices]
    heights = [p[2] for p in vertices]
    low_d, high_d = min(depths), max(depths)
    low_h, high_h = min(heights), max(heights)
    assert high_d - low_d > 1e-8 and high_h - low_h > 1e-8
    return [
        (
            0.02 + 0.96 * (p[1] - low_d) / (high_d - low_d),
            0.02 + 0.96 * (p[2] - low_h) / (high_h - low_h),
        )
        for p in vertices
    ]


def lower_ear_bridge(lower_id, root_id, return_id):
    return (lower_id, return_id, root_id)


def fit_ear_roots(shell):
    bpy.context.view_layer.update()
    evaluated = shell.evaluated_get(bpy.context.evaluated_depsgraph_get())
    data = evaluated.to_mesh()
    reports = []
    try:
        uses = {}
        for poly in data.polygons:
            ids = list(poly.vertices)
            for a, b in zip(ids, ids[1:] + ids[:1]):
                key = tuple(sorted((a, b)))
                uses[key] = uses.get(key, 0) + 1
        boundary_edges = {e for e, n in uses.items() if n == 1}
        boundary_ids = {v for e in boundary_edges for v in e}
        for side in [-1, 1]:
            roots = [
                i
                for i in boundary_ids
                if side * data.vertices[i].co.x > 0
                and C["ear_root_min_height"]
                <= (data.vertices[i].co.z - Y0) / H
                <= C["ear_root_max_height"]
                and abs(data.vertices[i].co.x) > width(data.vertices[i].co.z) - 0.015 * H
            ]
            roots.sort(key=lambda i: data.vertices[i].co.z)
            assert len(roots) >= 5, "Insufficient ear-root boundary samples"
            assert all(
                tuple(sorted((a, b))) in boundary_edges for a, b in zip(roots, roots[1:])
            ), "Ear root must be a connected facial boundary chain"
            root_points = [data.vertices[i].co.copy() for i in roots]
            low, high = root_points[0].z, root_points[-1].z
            vs, fs, rows, uvpoints = [], [], [], []
            root_indices = []
            angular_source_fan = False
            budget_stations = source_budget_fan_stations(root_points,low,high,side) if STYLE.get("sampling_ear_budget",0) else None
            for j, root in enumerate(root_points):
                t = (root.z - low) / (high - low)
                row = []
                values = [0] if j in [0, len(root_points) - 1] else [0, 0.22, 0.5, 0.70, 0.88, 1]
                if budget_stations is not None and j in (1,2):
                    values = budget_stations
                    angular_source_fan = True
                elif j == 1 and STYLE.get("sampling_first_pass", 0):
                    values = lower_fan_stations(root, root_points[0], t, low, high, side)
                    angular_source_fan = True
                for u in values:
                    outward, posterior = membrane_offset(t, u, C, (-root.y - Z0) / H)
                    row.append(len(vs))
                    lift = (ear_arc_height(t, low, high) - root.z) * sqrt(max(0, sin(pi * u / 2)))
                    vs.append((root.x + side * H * outward, root.y + H * posterior, root.z + lift))
                    uvpoints.append((0.02 + 0.96 * u, 0.02 + 0.96 * t))
                root_indices.append(row[0])
                rows.append(row)
            for a, b in zip(rows, rows[1:]):
                if len(a) == 1:
                    fs.extend((a[0], b[k + 1], b[k]) for k in range(len(b) - 1))
                elif len(b) == 1:
                    fs.extend((a[k], a[k + 1], b[0]) for k in range(len(a) - 1))
                elif len(a) != len(b):
                    fs.extend(source_row_transition(a,b,vs))
                elif budget_stations is not None and len(a)==4:
                    for k in range(len(a)-1): fs.extend(source_quad_split(a[k:k+2],b[k:k+2],vs))
                else:
                    fs.extend((a[k], a[k + 1], b[k + 1], b[k]) for k in range(len(a) - 1))
            main_face_count = len(fs)
            # A narrow skin return lip supports back-side visibility without
            # closing the ear basin or creating an independent dark stroke.
            rim = []
            crown = []
            for j, row in enumerate(rows):
                outer = row[-1]
                if j in [0, len(rows) - 1]:
                    rim.append(outer)
                    crown.append(outer)
                    continue
                t = (root_points[j].z - low) / (high - low)
                end_weight = min(
                    1, t / C["ear_return_end_fade"], (1 - t) / C["ear_return_end_fade"]
                )
                end_weight = end_weight * end_weight * (3 - 2 * end_weight)
                x, y, z = vs[outer]
                return_width = min(
                    H * C["ear_return_lip_width"] * sin(pi * t) * end_weight,
                    abs(x - root_points[j].x)
                    * min(
                        C["ear_return_basin_width_fraction"],
                        curve(t, 1, C["ear_return_width_profile"]),
                    ),
                )
                return_depth = min(
                    H * C["ear_return_anterior_depth"] * sin(pi * t) * end_weight,
                    max(0, y - root_points[j].y) * C["ear_return_basin_depth_fraction"],
                )
                rim.append(len(vs))
                vs.append(
                    (
                        x - side * return_width,
                        y - return_depth,
                        z,
                    )
                )
                uvpoints.append((0.86, 0.02 + 0.96 * t))
                crown.append(len(vs))
                vs.append(return_crown_point(vs[outer],vs[rim[-1]],t,C["ear_return_crown_m"]))
                uvpoints.append((0.93,0.02 + 0.96*t))
            for j in range(len(rows) - 1):
                for first,second in [([r[-1] for r in rows],crown),(crown,rim)]:
                    corners = [first[j],second[j],second[j+1],first[j+1]]
                    clean = list(dict.fromkeys(corners))
                    if len(clean) >= 3:
                        fs.append(tuple(clean))
            # Join the lower rear wrap to the neighboring open facial edge.
            # The anterior root chain remains unchanged; no third face is added
            # to its already connected edges.
            lower_neighbors = [
                b if a == roots[0] else a
                for a, b in boundary_edges
                if roots[0] in (a, b) and data.vertices[b if a == roots[0] else a].co.z < low
            ]
            assert len(lower_neighbors) == 1, "Need one lower facial boundary neighbor"
            lower = data.vertices[lower_neighbors[0]].co.copy()
            lower_id = len(vs)
            vs.append(tuple(lower))
            bridge = lower_ear_bridge(lower_id, rows[0][0], rim[1])
            # Oppose the existing root->rim edge. A world-y normal heuristic
            # incorrectly flipped both shared edges of this lower bridge.
            fs.append(bridge)
            # Mirror the lower attachment at the upper root. Without this
            # bridge the return lip stops below the neighboring face edge,
            # leaving the visible notch marked in the side/back wireframe.
            upper_neighbors = [
                b if a == roots[-1] else a
                for a, b in boundary_edges
                if roots[-1] in (a, b)
                and data.vertices[b if a == roots[-1] else a].co.z > high
            ]
            assert len(upper_neighbors) == 1, "Need one upper facial boundary neighbor"
            upper = data.vertices[upper_neighbors[0]].co.copy()
            upper_id = len(vs)
            vs.append(tuple(upper))
            fs.append((upper_id, rows[-1][0], rim[-2]))
            uvpoints = project_ear_uv(vs)
            if side < 0:
                fs = [tuple(reversed(f)) for f in fs]
            # A single wrapped surface: the inward return has a free inner edge.
            # Do not duplicate the whole front sheet into a nearly coincident back.
            front_face_count = len(fs)
            obj = mesh("Ear hollow pinna " + str(side), vs, fs, SKIN)
            assert all(poly.area > 1e-12 for poly in obj.data.polygons), "Degenerate ear membrane"
            reverse_measure = sum(
                poly.normal.x * side * poly.area
                for poly in list(obj.data.polygons)[main_face_count:]
            )
            assert reverse_measure < 0, "Ear return must wrap to the reverse lateral side"
            obj["ear_return_side_normal_measure"] = reverse_measure
            obj["ear_detail"] = True
            obj["ear_return_band_faces"] = 2*(len(rows)-1)
            obj["ear_main_faces"] = main_face_count
            obj["ear_front_faces"] = front_face_count
            detail = obj.data.uv_layers.new(name="DetailUV")
            for loop in obj.data.loops:
                detail.data[loop.index].uv = uvpoints[loop.vertex_index]
            delta = obj.data.attributes.new("MouthOpenDelta", "FLOAT_VECTOR", "POINT")
            for value in delta.data:
                value.vector = (0, 0, 0)
            error = max(
                (obj.data.vertices[i].co - r).length for i, r in zip(root_indices, root_points)
            )
            assert error < 1e-7, "Ear-root seam must match actual evaluated skin"
            reports.append(
                {
                    "name": obj.name,
                    "source_fan_policy": "quality-budget-source-fan-v2" if budget_stations is not None else "angular-source-fan-v1" if angular_source_fan else None,
                    "root_vertices": len(roots),
                    "return_side_normal_measure": reverse_measure,
                    "max_root_position_error_m": error,
                    "root_positions": [list(r) for r in root_points],
                    "lower_bridge_root_position": list(lower),
                    "lower_bridge_triangle_positions": [list(vs[i]) for i in bridge],
                    "geometry": "open-backed shallow membrane; no separate closed back shell",
                }
            )
    finally:
        evaluated.to_mesh_clear()
    return reports


def verify_ear_seams(data, reports):
    from mathutils.kdtree import KDTree

    tree = KDTree(len(data.vertices))
    for vertex in data.vertices:
        tree.insert(vertex.co, vertex.index)
    tree.balance()
    uses = {}
    for poly in data.polygons:
        ids = list(poly.vertices)
        for a, b in zip(ids, ids[1:] + ids[:1]):
            key = tuple(sorted((a, b)))
            uses[key] = uses.get(key, 0) + 1
    out = []
    for report in reports:
        ids = []
        for x, y, z in report["root_positions"]:
            _, index, distance = tree.find(Vector((x, y, z - 0.021)))
            assert distance < 0.000002, "Missing joined ear-root vertex"
            ids.append(index)
        counts = [uses.get(tuple(sorted((a, b))), 0) for a, b in zip(ids, ids[1:])]
        assert all(c == 2 for c in counts), "Ear-root edges must connect exactly two surfaces"
        out.append(
            {
                "name": report["name"],
                "shared_manifold_root_edges": len(counts),
                "edge_face_usage": sorted(set(counts)),
            }
        )
    return out


def return_crown_point(outer,inner,t,amplitude):
    """A shallow posterior crown inside the existing open return; endpoints fixed."""
    from math import sin,pi
    fade=sin(pi*max(0,min(1,t)))**2
    return tuple((outer[k]+inner[k])/2+(amplitude*fade if k==1 else 0) for k in range(3))


"""Bounded source-stage angular sampling of the lower ear endpoint fan."""

def lower_fan_stations(root, endpoint, t, low, high, side):
    """Place five sectors by angular arc length on the unchanged membrane curve."""
    from math import acos, isfinite
    def point(u):
        outward, posterior = membrane_offset(t, u, C, (-root.y-Z0)/H)
        lift = (ear_arc_height(t, low, high)-root.z)*sqrt(max(0, sin(pi*u/2)))
        if 'researched_ear_kind' in C:
            lift += H*researched_ear_tip_lift(t, u, C)
        return (root.x+side*H*outward, root.y+H*posterior, root.z+lift)
    directions = []
    for i in range(65):
        p = point(i/64)
        if not all(isfinite(value) for value in p):
            raise ValueError("Nonfinite source ear fan point")
        v = [p[k]-endpoint[k] for k in range(3)]
        length = sqrt(sum(x*x for x in v))
        if length < 1e-8:
            raise ValueError('Degenerate source ear fan ray')
        directions.append([x/length for x in v])
    cumulative = [0.]
    for a,b in zip(directions,directions[1:]):
        cumulative.append(cumulative[-1]+acos(max(-1.,min(1.,sum(x*y for x,y in zip(a,b))))))
    if cumulative[-1] < .05:
        raise ValueError('Insufficient source ear fan angular span')
    stations = [0.]
    for fraction in (.2,.4,.6,.8):
        target = cumulative[-1]*fraction
        for i,(a,b) in enumerate(zip(cumulative,cumulative[1:])):
            if a <= target <= b and b-a > 1e-12:
                stations.append((i+(target-a)/(b-a))/64)
                break
    stations.append(1.)
    if len(stations)!=6 or any(b-a < 1e-4 for a,b in zip(stations,stations[1:])):
        raise ValueError('Collapsed source ear fan sampling')
    return stations


def source_triangle_score(points):
    from math import acos, degrees, sqrt, isfinite
    if len(points)!=3 or any(len(p)!=3 or any(not isfinite(c) for c in p) for p in points): return 0.0
    edges = [[points[(i+1)%3][k]-points[i][k] for k in range(3)] for i in range(3)]
    lengths = [sqrt(sum(x*x for x in edge)) for edge in edges]
    if min(lengths) < 1e-9: return 0.0
    angles = [degrees(acos(max(-1., min(1., (lengths[i]**2+lengths[(i+2)%3]**2-lengths[(i+1)%3]**2)/(2*lengths[i]*lengths[(i+2)%3]))))) for i in range(3)]
    if min(angles) < 12 or max(angles) > 145: return 0.0
    a,b=edges[:2];cross=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
    return 2*sqrt(3)*sqrt(sum(x*x for x in cross))/sum(x*x for x in lengths)


def source_quad_split(a, b, vertices):
    options=[[(a[0],a[1],b[1]),(a[0],b[1],b[0])],[(a[0],a[1],b[0]),(a[1],b[1],b[0])]]
    return max(options,key=lambda faces:min(source_triangle_score([vertices[i] for i in f]) for f in faces))


def source_budget_fan_stations(roots, low, high, side):
    def point(j,u):
        root=roots[j];t=(root.z-low)/(high-low)
        outward,posterior=membrane_offset(t,u,C,(-root.y-Z0)/H)
        lift=(ear_arc_height(t,low,high)-root.z)*sqrt(max(0,sin(pi*u/2)))
        if 'researched_ear_kind' in C: lift += H*researched_ear_tip_lift(t,u,C)
        return (root.x+side*H*outward,root.y+H*posterior,root.z+lift)
    count=129;grid=[i/(count-1) for i in range(count)]
    first=[point(1,u) for u in grid];second=[point(2,u) for u in grid]
    score=[[0.]*count for _ in grid]
    for i in range(count):
        for j in range(i+1,count):
            fan=source_triangle_score([roots[0],first[i],first[j]])
            if not fan: continue
            vertices=[first[i],first[j],second[i],second[j]]
            faces=source_quad_split([0,1],[2,3],vertices)
            score[i][j]=min(fan,*(source_triangle_score([vertices[k] for k in f]) for f in faces))
    dp=[[0.]*count for _ in range(4)];back=[[0]*count for _ in range(4)];dp[0][0]=1.
    for k in range(1,4):
        for j in range(k,count):
            choices=[min(dp[k-1][i],score[i][j]) for i in range(j)]
            best=max(range(j),key=lambda i:choices[i]);dp[k][j]=choices[best];back[k][j]=best
    if dp[3][-1] < .35: raise ValueError('No source three-sector fan meets quality budget')
    ids=[count-1]
    for k in range(3,0,-1):ids.append(back[k][ids[-1]])
    return [grid[i] for i in reversed(ids)]


def source_row_transition(a, b, vertices):
    dp={(0,0):1.};back={}
    for i in range(len(a)):
        for j in range(len(b)):
            for di,dj in ((1,0),(0,1)):
                ni,nj=i+di,j+dj
                if ni>=len(a) or nj>=len(b):continue
                face=(a[i],a[ni],b[j]) if di else (a[i],b[nj],b[j])
                quality=min(dp.get((i,j),0.),source_triangle_score([vertices[k] for k in face]))
                if quality>dp.get((ni,nj),0.):dp[ni,nj]=quality;back[ni,nj]=(i,j,face)
    end=(len(a)-1,len(b)-1)
    if dp.get(end,0.)<.35:raise ValueError('No conforming ear transition meets quality budget')
    faces=[]
    while end!=(0,0):i,j,face=back[end];faces.append(face);end=(i,j)
    return list(reversed(faces))
