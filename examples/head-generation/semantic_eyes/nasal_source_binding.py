"""Bind a relative nasal patch to this build's validated authoring surface."""
from collections import Counter
import numpy as np
from nasal_source_geometry import edge_incidence, patch_boundary
from nasal_source_template import TEMPLATE


def validate_domain(vertices, faces, selected, boundary, edge_groups):
    """Reject topology drift rather than accidentally binding another patch."""
    if len(selected) != TEMPLATE['domain_faces'] or len(boundary) != TEMPLATE['boundary_count']:
        raise ValueError('Nasal source domain cardinality changed')
    all_edges = edge_incidence(faces)
    valence = Counter(i for edge in all_edges for i in edge)
    sub_edges = edge_incidence([faces[i] for i in sorted(selected)])
    sub_valence = Counter(i for edge in sub_edges for i in edge)
    if [valence[i] for i in boundary] != TEMPLATE['source_valence']:
        raise ValueError('Nasal source global boundary valence changed')
    if [sub_valence[i] for i in boundary] != TEMPLATE['domain_valence']:
        raise ValueError('Nasal source local boundary valence changed')
    if [k for k,i in enumerate(boundary) if abs(vertices[i,0]) < 1e-8] != TEMPLATE['axis_slots']:
        raise ValueError('Nasal source central-axis ownership changed')
    expected = []
    for a,b in zip(boundary,boundary[1:]+boundary[:1]):
        edge = tuple(sorted((a,b)))
        if len(all_edges[edge]) != 2 or edge not in edge_groups:
            raise ValueError('Nasal boundary must retain a two-sided source edge')
        expected.append(edge_groups[edge])
    if expected != TEMPLATE['boundary_groups']:
        raise ValueError('Nasal protected crease-group ownership changed')


def bind_patch(surface, vertices, core, core_lineage, boundary):
    """Reconstruct only normalized local slots; never consume old point IDs."""
    points = list(core.copy())
    lineage = list(core_lineage)
    def position(ref):
        kind,index = ref
        if kind == 'outer':
            return vertices[boundary[index]]
        if kind == 'core' and 0 <= index < len(core):
            return points[index]
        if kind == 'center' and 0 <= index < len(points)-len(core):
            return points[len(core)+index]
        raise ValueError('Invalid relative nasal source slot')
    for center in TEMPLATE['centers']:
        weights = np.asarray(center['weights'],dtype=float)
        if len(weights) != 3 or min(weights) < 0 or abs(sum(weights)-1) > 1e-9:
            raise ValueError('Invalid nasal transition construction weights')
        xy = weights@np.asarray([position(ref)[[0,2]] for ref in center['parents']])
        ids,shares,point = surface.lineage(*xy)
        points.append(point); lineage.append((ids,shares))
    # Local indices: fixed outer boundary, then new core and transition points.
    def index(ref):
        kind,slot = ref
        position(ref)
        return slot if kind == 'outer' else len(boundary)+slot+(len(core) if kind == 'center' else 0)
    quads = [tuple(index(ref) for ref in face) for face in TEMPLATE['quads']]
    new_points = np.asarray(points,dtype=float).reshape(-1,3)
    total = np.r_[vertices[boundary],new_points]
    edges = edge_incidence(quads)
    expected = {tuple(sorted((i,(i+1)%len(boundary)))) for i in range(len(boundary))}
    if patch_boundary(quads,range(len(quads))) != expected:
        raise ValueError('Nasal source template does not conform to current boundary')
    directions = Counter((a,b) for face in quads for a,b in zip(face,face[1:]+face[:1]))
    if any(len(owners)>2 or (len(owners)==2 and (directions[a,b]!=1 or directions[b,a]!=1))
           for (a,b),owners in edges.items()):
        raise ValueError('Nasal source patch winding or incidence is inconsistent')
    if len(total)-len(edges)+len(quads) != 1:
        raise ValueError('Nasal source template must remain a disk')
    if len({tuple(sorted(q)) for q in quads}) != len(quads):
        raise ValueError('Duplicate nasal source face')
    return new_points,quads,lineage


def validate_patch(surface, boundary_points, new_points, quads):
    """Float32 export-aware geometry gates for a bounded visual trial."""
    points = np.r_[boundary_points,new_points].astype(np.float32).astype(float)
    bary = np.asarray([(1/3,1/3,1/3),(.6,.2,.2),(.2,.6,.2),(.2,.2,.6),
                       (.45,.45,.1),(.45,.1,.45),(.1,.45,.45)])
    angles,errors = [],[]
    for face in quads:
        q = points[list(face)]
        for i in range(4):
            a,b = q[(i-1)%4]-q[i],q[(i+1)%4]-q[i]
            norm = np.linalg.norm(a)*np.linalg.norm(b)
            if norm < 1e-12:
                raise ValueError('Collapsed nasal source edge')
            angles.append(float(np.degrees(np.arccos(np.clip(a@b/norm,-1,1)))))
        for tri in ((0,1,2),(0,2,3),(1,2,3),(1,3,0)):
            t = q[list(tri)]
            xy = t[:,[0,2]]
            a,b = xy[1]-xy[0],xy[2]-xy[0]
            if a[0]*b[1]-a[1]*b[0] <= 1e-12:
                raise ValueError('Nasal source quad is folded or degenerate')
            for sample in bary@t:
                target = surface.sample(sample[0],sample[2])
                errors.append(abs(float(sample[1]-target[1])))
    if min(angles)<10 or max(angles)>170 or max(errors)>.00100001:
        raise ValueError('Nasal source trial angle or author-surface budget exceeded')
    return {'min_angle_degrees':min(angles),'max_angle_degrees':max(angles),
            'max_author_surface_error_mm':max(errors)*1000,'samples':len(errors),
            'visual_trial_only':True}
