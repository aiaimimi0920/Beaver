"""Bounded coupling of the shared cheek frame, not the functional eyelid."""
import numpy as np
from nasal_source_binding import validate_patch
from nasal_source_template import TEMPLATE

BOUNDARY_SLOTS = (18,19,21,22)
BOUNDARY_CAPS = np.array((.001,.001,.001,.0005))
BOUNDARY_GRAPH_DISTANCE = (4,3,3,2)
BOUNDARY_OFFSETS = np.array(((.0011594551311814895,.02112401129451939),
    (-.008636606855906635,-.027775955244371053),
    (.025878402717031564,.008890863434096865),
    (-.012479986007303826,-.005309945080840659)))


def boundary_patch_metrics(points, quads):
    """Extrema of the complete affected patch, not per-face Pareto locks."""
    angles,ratios = [],[]
    for face in quads:
        p = points[list(face)]
        lengths = np.linalg.norm(p-np.roll(p,1,axis=0),axis=1)
        if min(lengths) <= 1e-8:
            raise ValueError('Collapsed coupled cheek edge')
        ratios.append(float(max(lengths)/min(lengths)))
        for i in range(4):
            a,b = p[(i-1)%4]-p[i],p[(i+1)%4]-p[i]
            angles.append(float(np.degrees(np.arccos(np.clip(
                a@b/np.linalg.norm(a)/np.linalg.norm(b),-1,1)))))
    return {'min_angle':min(angles),'max_angle':max(angles),'max_edge_ratio':max(ratios)}


def boundary_aperture_distances(features):
    distance = {i:0 for i in features['apertures']}
    front = set(distance)
    for level in range(1,5):
        next_front = {n for i in front for n in features['skin_neighbors'][i] if n not in distance}
        for i in next_front:
            distance[i] = level
        front = next_front
    return distance


def couple_nasal_boundary(surface, vertices, faces, regions, selected, boundary,
                          points, quads, lineage, features, edge_groups):
    """Project source-relative offsets and validate both sides of the old seam.

    Original author surface and core are immutable. Baseline core is rounded
    exactly as a stored mesh before dependent centers are reconstructed.
    """
    ids = [boundary[s] for s in BOUNDARY_SLOTS]
    distances = boundary_aperture_distances(features)
    protected = vertices[sorted(features['protected'])]
    for i,cap,level in zip(ids,BOUNDARY_CAPS,BOUNDARY_GRAPH_DISTANCE):
        if (i in features['protected'] or features['membership'][i] != {1}
                or distances.get(i) != level):
            raise ValueError('Coupled cheek slot is not the expected nonfunctional frame')
        if np.min(np.linalg.norm(protected-vertices[i],axis=1)) < 9*cap:
            raise ValueError('Coupled cheek frame is too close to a protected feature')
        if any(value for edge,value in edge_groups.items() if i in edge):
            raise ValueError('Cannot move an authored cheek crease group')
    span = float(features['eye_height']-features['nose_peak'][2])
    if not .01 < span < .06:
        raise ValueError('Invalid current nasal-to-iris source span')
    baseline_points = np.asarray(points,dtype=np.float32).astype(float)
    new_points = baseline_points.copy()
    border = vertices[boundary].copy()
    for slot,offset in zip(BOUNDARY_SLOTS,BOUNDARY_OFFSETS):
        xy = border[slot,[0,2]]+span*offset
        border[slot] = surface.sample(*xy)
    dependent = [i for i,c in enumerate(TEMPLATE['centers'])
                 if any(kind=='outer' and k in BOUNDARY_SLOTS for kind,k in c['parents'])]
    if dependent != [0,2]:
        raise ValueError('Coupled cheek transition ownership changed')
    for index in dependent:
        center = TEMPLATE['centers'][index]
        parents = []
        for kind,k in center['parents']:
            if kind not in ('outer','core'):
                raise ValueError('Unexpected coupled center dependency')
            parents.append(border[k] if kind=='outer' else baseline_points[k])
        xy = np.asarray(center['weights'])@np.asarray(parents)[:,[0,2]]
        new_points[103+index] = surface.sample(*xy)
    border = border.astype(np.float32).astype(float)
    new_points = new_points.astype(np.float32).astype(float)
    moves = np.linalg.norm(border[list(BOUNDARY_SLOTS)]-vertices[ids],axis=1)
    if np.any(moves>BOUNDARY_CAPS):
        raise ValueError('Stored coupled cheek motion exceeds its exact cap')
    changed = vertices.copy()
    changed[ids] = border[list(BOUNDARY_SLOTS)]
    migrations = {}
    for i,slot in zip(ids,BOUNDARY_SLOTS):
        donor,weights,_ = surface.lineage(*border[slot,[0,2]])
        migrations[i] = {'point':border[slot].copy(),'lineage':(donor,weights)}
    lineage = list(lineage)
    for index in dependent:
        donor,weights,_ = surface.lineage(*new_points[103+index,[0,2]])
        lineage[103+index] = (donor,weights)
    outside = [i for i,f in enumerate(faces) if i not in selected and set(f)&set(ids)]
    if len(outside)!=7 or any(regions[i]!=1 or len(faces[i])!=4 for i in outside):
        raise ValueError('Coupled cheek outside ownership changed')
    n = len(vertices)
    def global_index(i):
        return boundary[i] if i<len(boundary) else n+i-len(boundary)
    added = [tuple(global_index(i) for i in q) for q in quads]
    affected_points = set(ids)|{n+103+i for i in dependent}
    affected = [faces[i] for i in outside]+[q for q in added if set(q)&affected_points]
    if len(affected)!=15:
        raise ValueError('Coupled cheek must validate the complete inside/outside patch')
    before = np.r_[vertices,baseline_points].astype(np.float32).astype(float)
    after = np.r_[changed,new_points].astype(np.float32).astype(float)
    prior,current = boundary_patch_metrics(before,affected),boundary_patch_metrics(after,affected)
    if (current['min_angle']<prior['min_angle'] or current['max_angle']>prior['max_angle']
            or current['max_edge_ratio']>prior['max_edge_ratio']):
        raise ValueError('Stored coupled cheek patch extrema regressed')
    quality = validate_patch(surface,after,np.empty((0,3)),affected)
    if not np.array_equal(new_points[:103],baseline_points[:103]):
        raise ValueError('Coupled frame changed a primary nasal guide or support')
    return new_points,lineage,migrations,{'version':'nasal-cheek-boundary-v1',
        'boundary_slots':list(BOUNDARY_SLOTS),'motion_mm':(moves*1000).tolist(),
        'baseline_extrema':prior,'stored_extrema':current,'quality':quality,
        'outside_quads':len(outside),'affected_quads':len(affected),
        'constraint':'Patch extrema; not per-face nonregression',
        'functional_features_and_core':'unchanged'}


def snapshot_nasal_fields(bm, parents, migrations):
    """Immutable donors prevent order-dependent updates to shared old nodes."""
    result = {}
    for target,row in migrations.items():
        ids,shares = row['lineage']
        weights = {int(i):float(w) for i,w in zip(ids,shares) if w>1e-12}
        if abs(sum(weights.values())-1)>1e-8:
            raise ValueError('Invalid coupled cheek field lineage')
        fields,deform = [],[]
        for kind in ('float','float_vector','color','float_color','shape','int','string'):
            accessor = getattr(bm.verts.layers,kind,None)
            if accessor is None:
                continue
            for layer in accessor.values():
                values = {i:parents[i][layer] for i in weights}
                first = next(iter(values.values()))
                if kind in ('int','string'):
                    if any(value!=first for value in values.values()):
                        raise ValueError('Coupled cheek crosses a discrete point-field boundary')
                    value = first
                elif kind == 'float':
                    value = sum(weights[i]*values[i] for i in weights)
                else:
                    value = tuple(sum(weights[i]*values[i][k] for i in weights) for k in range(len(first)))
                fields.append((layer,value))
        for layer in bm.verts.layers.deform.values():
            keys = set().union(*(set(parents[i][layer].keys()) for i in weights))
            deform.append((layer,{k:sum(weights[i]*parents[i][layer].get(k,0) for i in weights) for k in keys}))
        result[target] = (fields,deform)
    return result


def apply_nasal_fields(vertex, snapshot):
    """Use captured layer handles without relying on wrapper-object equality."""
    fields,deform = snapshot
    for layer,value in fields:
        vertex[layer] = value
    for layer,values in deform:
        vertex[layer].clear()
        for key,value in values.items():
            vertex[layer][key] = value
