"""Bounded final-triangle fairing inside the source generator, before normals/export."""


def fair_final_source_triangles(data, config, height, origin, assembly_offset, maximum_move=.0015, project_uv=None):
    """Reduce local geometric ripples without adding or deleting vertices/faces.

    Designed feature support, open boundaries, region seams and ear-root joins
    stay fixed. Every accepted step improves a triangle-adjacency objective and
    preserves finite area, original orientation and minimum-angle bounds.
    """
    from math import acos, cos, isfinite, pi, sqrt
    from struct import pack, unpack
    assert isfinite(maximum_move) and maximum_move>0, "Invalid source fairing displacement budget"
    assert isfinite(height) and height>0, "Invalid source face height"

    def sub(a, b):
        return tuple(x-y for x, y in zip(a, b))

    def add(a, b):
        return tuple(x+y for x, y in zip(a, b))

    def scale(a, value):
        return tuple(x*value for x in a)

    def dot(a, b):
        return sum(x*y for x, y in zip(a, b))

    def length(a):
        return sqrt(dot(a, a))

    def cross(a, b):
        return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])

    def geometry(points, face):
        a, b, c = (points[i] for i in face)
        raw = cross(sub(b, a), sub(c, a))
        magnitude = length(raw)
        if magnitude <= 2e-12:
            return None
        angles = []
        for index in range(3):
            p = points[face[index]]
            u = sub(points[face[(index+1)%3]], p)
            v = sub(points[face[(index+2)%3]], p)
            angles.append(acos(max(-1., min(1., dot(u, v)/(length(u)*length(v))))))
        return scale(raw, 1/magnitude), magnitude/2, min(angles)

    original = [tuple(v.co) for v in data.vertices]
    faces = [tuple(p.vertices) for p in data.polygons]
    assert original and faces and all(isfinite(c) for point in original for c in point), "Invalid source fairing mesh"
    assert all(len(face)==3 for face in faces), 'Final source fairing requires triangles'
    regions = data.attributes['NormalRegion']
    region = [item.value for item in regions.data]
    generated = data.attributes.get('OutlineGeneratedReturn')
    returned = {i for i in range(len(faces)) if generated and generated.data[i].value}
    neighbors = [set() for _ in original]
    incident = [set() for _ in original]
    memberships = [set() for _ in original]
    edge_faces = {}
    for face_id, face in enumerate(faces):
        for vertex in face:
            incident[vertex].add(face_id)
            memberships[vertex].add(region[face_id])
        for a, b in zip(face, face[1:]+face[:1]):
            neighbors[a].add(b)
            neighbors[b].add(a)
            edge_faces.setdefault(tuple(sorted((a,b))), []).append(face_id)
    baseline = [geometry(original, face) for face in faces]
    boundary = {i for edge, owners in edge_faces.items() if len(owners)!=2 for i in edge}
    pinned = set(boundary)
    # First repair the continuous front skin, not intentional underside folds.
    for face_id, face in enumerate(faces):
        if region[face_id]!=1 or face_id in returned or baseline[face_id] is None or baseline[face_id][0][1]>=-.05:
            pinned.update(face)
    for owners in edge_faces.values():
        if len(owners)==2 and all(baseline[i] for i in owners):
            if dot(baseline[owners[0]][0],baseline[owners[1]][0])<cos(80*pi/180):
                pinned.update(i for f in owners for i in faces[f])
    for i, point in enumerate(original):
        if len(memberships[i])!=1 or not memberships[i].issubset({1,2}) or abs(point[0])<1e-7:
            pinned.add(i)
        if incident[i].intersection(returned):
            pinned.add(i)
        v = (point[2]-assembly_offset-origin)/height
        nose = config['nose_stations']
        if nose[0][0]-.015 < v < nose[-1][0]+.015 and abs(point[0]) < height*max(row[2] for row in nose)+.003:
            pinned.add(i)
        if abs(v-config['mouth_height'])<.045 and abs(point[0])<height*config['mouth_half_width']+.006:
            pinned.add(i)
    groups = data.attributes.get('FacialCreaseGroup')
    if groups:
        for edge in data.edges:
            if groups.data[edge.index].value:
                pinned.update(edge.vertices)
    # One ring around open apertures, not every intentional regional seam.
    pinned.update(j for i in boundary for j in neighbors[i])
    pairs = {}
    buckets = {}
    for i, p in enumerate(original):
        buckets.setdefault((round(abs(p[0]),6), round(p[1],6), round(p[2],6)), []).append(i)
    for ids in buckets.values():
        if len(ids)==2:
            a, b = ids
            if original[a][0]*original[b][0]<0:
                pairs[a]=b
                pairs[b]=a
    pinned.update(i for i in range(len(original)) if i not in pairs)
    pinned.update(pairs[i] for i in list(pinned) if i in pairs)
    skin_edges = []
    seeds = set()
    for edge, owners in sorted(edge_faces.items()):
        if len(owners)!=2 or any(i in returned for i in owners):
            continue
        a, b = owners
        if region[a]!=1 or region[a]!=region[b] or not baseline[a] or not baseline[b]:
            continue
        skin_edges.append((edge,a,b))
        if dot(baseline[a][0],baseline[b][0]) < cos(20*pi/180):
            seeds.update(i for f in owners for i in faces[f] if i not in pinned)
    weights = {i:1. for i in seeds}
    front = set(seeds)
    for factor in (.5,.25):
        front = {j for i in front for j in neighbors[i] if j not in pinned and j not in weights}
        weights.update({i:factor for i in front})
    movable = set(weights)
    movable = {i for i in movable if pairs[i] in movable}
    touched = {f for i in movable for f in incident[i]}
    objective_edges = [(e,a,b) for e,a,b in skin_edges if a in touched or b in touched]

    last_invalid_faces=set()
    rejected_geometry=set()
    def evaluate(points):
        last_invalid_faces.clear()
        values = [geometry(points,face) if i in touched else baseline[i] for i,face in enumerate(faces)]
        for i in sorted(touched):
            value = values[i]
            if (value is None or value[1]<max(1e-12, baseline[i][1]*.2)
                or dot(value[0],baseline[i][0])<cos(25*pi/180)
                or value[2]+1e-8<min(baseline[i][2],5*pi/180)):
                last_invalid_faces.add(i)
        if last_invalid_faces:
            rejected_geometry.update(last_invalid_faces)
            return None
        angles = [acos(max(-1.,min(1.,dot(values[a][0],values[b][0])))) for _,a,b in objective_edges]
        energy = sum(angle*angle for angle in angles)
        return values, energy, max(angles,default=0.)

    current = list(original)
    state = evaluate(current)
    assert state is not None, 'Invalid source fairing baseline'
    initial_energy, initial_maximum = state[1:]
    accepted_steps = 0
    for _ in range(8):
        proposals = {}
        for i in sorted(movable):
            normal = (0.,0.,0.)
            for f in incident[i]:
                normal = add(normal,scale(state[0][f][0],state[0][f][1]))
            magnitude = length(normal)
            if magnitude<=1e-12:
                proposals[i]=(0.,0.,0.)
                continue
            normal=scale(normal,1/magnitude)
            local_edges=[(a,b) for _,a,b in objective_edges if a in incident[i] or b in incident[i]]
            def local_energy(distance):
                sample=list(current)
                sample[i]=add(sample[i],scale(normal,distance))
                value=0.
                for a,b in local_edges:
                    ga,gb=geometry(sample,faces[a]),geometry(sample,faces[b])
                    if ga is None or gb is None:
                        return None
                    value+=acos(max(-1.,min(1.,dot(ga[0],gb[0]))))**2
                return value
            epsilon=1e-5
            before,plus,minus=local_energy(0.),local_energy(epsilon),local_energy(-epsilon)
            if plus is None or minus is None:
                proposals[i]=(0.,0.,0.)
                continue
            gradient=(plus-minus)/(2*epsilon)
            curvature=(plus+minus-2*before)/(epsilon*epsilon)
            step=-gradient/max(curvature,abs(gradient)/.0003,1e-8)
            proposals[i]=scale(normal,step*weights[i])
        # Apply symmetric displacements; do not erase any original asymmetry.
        for i in sorted(movable):
            j=pairs[i]
            if i<j:
                a,b=proposals[i],proposals[j]
                average=((a[0]-b[0])/2,(a[1]+b[1])/2,(a[2]+b[2])/2)
                proposals[i]=average
                proposals[j]=(-average[0],average[1],average[2])
        accepted = False
        for fraction in (1.,.5,.25,.125,.0625):
            local_scale={i:1. for i in movable}
            for repair in range(12):
                trial=list(current)
                for i in movable:
                    point=add(current[i],scale(proposals[i],fraction*local_scale[i]))
                    delta=sub(point,original[i])
                    if length(delta)>maximum_move:
                        point=add(original[i],scale(delta,maximum_move/length(delta)))
                    trial[i]=unpack('fff',pack('fff',*point))
                if any(length(sub(trial[i],original[i]))>maximum_move+1e-7 for i in movable):
                    break
                if not all(isfinite(c) for i in movable for c in trial[i]):
                    break
                result=evaluate(trial)
                if result is None:
                    bad={i for f in last_invalid_faces for i in faces[f] if i in movable}
                    bad.update(pairs[i] for i in list(bad))
                    for i in bad:
                        local_scale[i]=local_scale[i]*.5 if repair<8 else 0.
                    continue
                if result[1]<state[1]-1e-10 and result[2]<=state[2]+1e-8:
                    current,state=trial,result
                    accepted_steps+=1
                    accepted=True
                break
            if accepted:
                break
        if not accepted:
            break
    assert all(current[i]==original[i] for i in pinned), 'Protected feature moved'
    assert len(current)==len(data.vertices) and len(faces)==len(data.polygons)
    for vertex, point in zip(data.vertices,current):
        vertex.co=point
    data.update()
    stored=[tuple(v.co) for v in data.vertices]
    assert stored==current,'Stored fairing coordinates differ from validated float32 trial'
    stored_state=evaluate(stored)
    assert stored_state is not None and stored_state[1]<=initial_energy+1e-10,'Stored fairing geometry failed final gates'
    state=stored_state
    projected_corners=0
    if hasattr(data,'uv_layers'):
        uv2=data.uv_layers.get('UV2')
        assert uv2 is not None and callable(project_uv),'Final fairing needs the source face UV2 projection'
        for loop in data.loops:
            i=loop.vertex_index
            if stored[i]==original[i]:
                continue
            uv=project_uv(stored[i][0],stored[i][2]-assembly_offset)
            assert len(uv)==2 and all(isfinite(x) and 0<=x<=1 for x in uv),'Invalid final projected UV2'
            uv2.data[loop.index].uv=uv
            projected_corners+=1
    return {'topology_unchanged':True,'uv2_reprojected_corners':projected_corners,'material_uv_unchanged':True,'proposal':'bounded local dihedral-energy derivative','rejected_geometry_faces':sorted(rejected_geometry), 'selected_vertices':len(movable),
            'moved_vertices':sum(length(sub(a,b))>1e-9 for a,b in zip(current,original)),
            'maximum_displacement_m':max((length(sub(a,b)) for a,b in zip(current,original)),default=0.),
            'accepted_steps':accepted_steps,'coordinate_storage':'float32 checked before and after assignment','displacement_roundoff_tolerance_m':1e-7,'adjacency_energy_before':initial_energy,
            'adjacency_energy_after':state[1], 'maximum_patch_dihedral_before_degrees':initial_maximum*180/pi,
            'maximum_patch_dihedral_after_degrees':state[2]*180/pi,
            'protected_features_unchanged':True,'scope':'Continuous front source skin only; underside, hard folds and semantic features pinned; not collision or global surface-quality proof'}
