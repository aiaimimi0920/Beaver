"""Bounded graph/held-angle closure for neutral/current-locus support bands."""
from forehead_pattern_materialize import forehead_pattern_plain_records
import numpy as np
from orbital_support_rule import orbital_quad_quality
from forehead_star_topology import forehead_edge_counts

def forehead_band_angle_closure(proposal,seed_edges,maximum_edges=4,maximum_faces=5):
    """Extend opposite rails only when one-cut transition splits a held<50°corner.
    Does not move any pin or infer geometric feasibility from combinatorics.
    Adjacent cut patterns, boundaries, guide cuts and excess budget fail closed.
    All faceparent/locus proofs remain mandatory in the materializer.
    """
    proposal = forehead_pattern_plain_records(proposal)
    seed_edges = forehead_pattern_plain_records(seed_edges)
    faces=proposal['faces'];points=np.asarray(proposal['vertices']);selected={tuple(sorted(e))for e in seed_edges};held=set(proposal['protected'])|{i for path in proposal['paths'].values()for i in path};guide_edges={tuple(sorted(e))for path in proposal['paths'].values()for e in zip(path,path[1:])};ec=forehead_edge_counts(faces);held|={i for e,k in ec.items()if k==1 for i in e};trace=[]
    while True:
        additions=set();affected=[]
        for fi,q in enumerate(faces):
            if len(q)!=4:continue
            edges=[tuple(sorted(e))for e in zip(q,q[1:]+q[:1])];cuts=[k for k,e in enumerate(edges)if e in selected]
            if not cuts:continue
            affected.append(fi)
            if len(cuts)>2 or(len(cuts)==2 and(cuts[1]-cuts[0])%4!=2):raise ValueError('Closure creates unsupported adjacent/multiple cut mask')
            if len(cuts)==2:continue
            k=cuts[0];angles,_=orbital_quad_quality(points,np.asarray([q]));opposite=[(k+2)%4,(k+3)%4];blocked=[q[j]for j in opposite if q[j]in held and angles[0,j]<50.]
            if blocked:
                edge=edges[(k+2)%4]
                if ec[edge]!=2 or edge in guide_edges:raise ValueError('Held-corner closure hits boundary or guide rail')
                additions.add(edge);trace.append({'face':fi,'held_corners':blocked,'angles':[float(angles[0,q.index(i)])for i in blocked],'added_opposite_edge':list(edge)})
        if len(selected|additions)>maximum_edges or len(affected)>maximum_faces:raise ValueError('Bounded band closure budget exceeded')
        if not additions-selected:break
        selected|=additions
    if any(ec.get(e)!=2 or e in guide_edges for e in selected):raise ValueError('Invalid selected crossrail')
    return sorted(selected),{'trace':trace,'affected_faces':affected,'selected_edge_count':len(selected),'maximum_edges':maximum_edges,'maximum_faces':maximum_faces}
