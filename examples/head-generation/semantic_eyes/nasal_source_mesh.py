"""Atomic authoring-mesh replacement with current quad attribute lineage."""
import numpy as np
from scipy.spatial import cKDTree
from nasal_source_geometry import AuthorQuadSurface, canonical_boundary, edge_incidence
from nasal_source_domain import current_source_features, resolve_outer_anchors, select_source_domain
from nasal_source_core import make_core
from nasal_source_binding import validate_domain, bind_patch, validate_patch
from nasal_source_boundary import couple_nasal_boundary, snapshot_nasal_fields, apply_nasal_fields


def prepare_nasal_patch(vertices, faces, regions, ordinals, authored, edge_groups):
    """Resolve both sides before allocating or removing any source element."""
    features = current_source_features(vertices,faces,regions,ordinals)
    anchors = resolve_outer_anchors(vertices,regions,faces,authored,-.021)
    surface = AuthorQuadSurface(vertices,faces,regions)
    core,_,core_boundary,weights,cells,lines = make_core(surface,vertices,anchors,features)
    selected,boundary,domain = select_source_domain(vertices,faces,regions,cells,core[core_boundary],features)
    validate_domain(vertices,faces,selected,boundary,edge_groups)
    points,quads,lineage = bind_patch(surface,vertices,core,weights,boundary)
    base_points,base_lineage = points.copy(),list(lineage)
    points,lineage,migrations,transition = couple_nasal_boundary(
        surface,vertices,faces,regions,selected,boundary,base_points,quads,
        base_lineage,features,edge_groups)
    border = vertices[boundary].copy()
    for i,row in migrations.items():
        border[boundary.index(i)] = row['point']
    quality = validate_patch(surface,border,points,quads)
    distances,mirrors = cKDTree(vertices).query(vertices*np.array([-1,1,1]),k=2)
    used = {i for j in selected for i in faces[j]}
    if any(distances[i,0]>1e-7 or distances[i,1]<2e-6 for i in used):
        raise ValueError('Nasal source mirror has no unique matching current vertex')
    mirror = mirrors[:,0]
    lookup = {}
    for i,face in enumerate(faces):
        key = tuple(sorted(face))
        if key in lookup:
            raise ValueError('Duplicate original source face')
        lookup[key] = i
    mirrored = set()
    for i in selected:
        key = tuple(sorted(int(mirror[k]) for k in faces[i]))
        if key not in lookup or regions[lookup[key]] != 1:
            raise ValueError('Nasal source mirror has no matching skin quad')
        mirrored.add(lookup[key])
    other_boundary = canonical_boundary(vertices,faces,mirrored,side=-1)
    if other_boundary != [int(mirror[i]) for i in boundary] or selected&mirrored:
        raise ValueError('Nasal mirror domain is not an independent conforming half')
    validate_domain(vertices,faces,mirrored,other_boundary,edge_groups)
    reflected = vertices*np.array([-1,1,1])
    other_faces = [tuple(reversed(f)) for f in faces]
    other_surface = AuthorQuadSurface(reflected,other_faces,regions)
    other_lineage = [(tuple(int(mirror[i]) for i in ids),shares) for ids,shares in base_lineage]
    other_points,other_lineage,other_moves,other_transition = couple_nasal_boundary(
        other_surface,reflected,other_faces,regions,mirrored,other_boundary,
        base_points,quads,other_lineage,features,edge_groups)
    other_border = reflected[other_boundary].copy()
    for i,row in other_moves.items():
        other_border[other_boundary.index(i)] = row['point']
    if not np.array_equal(points,other_points) or not np.array_equal(border,other_border):
        raise ValueError('Coupled source frame lost exact mirror symmetry')
    other_quality = validate_patch(other_surface,other_border,other_points,quads)
    for row in other_moves.values():
        row['point'] = row['point']*[-1,1,1]
    return [dict(selected=selected,boundary=boundary,points=points,quads=quads,
                 lineage=lineage,migrations=migrations),
            dict(selected=mirrored,boundary=other_boundary,points=other_points*[-1,1,1],
                 quads=[q[::-1] for q in quads],lineage=other_lineage,migrations=other_moves)], {
        'domain':domain,'quality':[quality,other_quality],
        'shared_cheek_frame':[transition,other_transition],
        'primary_flow_count':len(lines),'primary_flow_local_paths':lines,
        'protected_current_vertices':len(features['protected']),
        'source_surface':'Current authoring quad bilinear surface; not export triangles',
        'scope':'Native visual trial, not final character acceptance'}


def replace_nasal_source(data, ordinals, authored):
    """All edits stay in a detached BMesh until every structural gate passes."""
    import bmesh
    from mathutils import Vector
    from face import depth
    from ear_topology import flow_point_fields, ear_loop_values, assign_ear_loop
    bm = bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.index_update(); bm.faces.index_update()
        bm.verts.ensure_lookup_table(); bm.faces.ensure_lookup_table()
        parents = list(bm.verts)
        original_faces = list(bm.faces)
        euler_before = len(bm.verts)-len(bm.edges)+len(bm.faces)
        open_edges_before = sum(len(e.link_faces)==1 for e in bm.edges)
        vertices = np.array([tuple(v.co) for v in parents])
        faces = [tuple(v.index for v in f.verts) for f in original_faces]
        region = bm.faces.layers.int.get('NormalRegion')
        crease = bm.edges.layers.float.get('crease_edge')
        group = bm.edges.layers.int.get('FacialCreaseGroup')
        if region is None or crease is None or group is None:
            raise ValueError('Nasal source semantic edge or face attributes are missing')
        groups = {tuple(sorted(v.index for v in e.verts)):int(e[group]) for e in bm.edges}
        specs,report = prepare_nasal_patch(vertices,faces,[f[region] for f in original_faces],
                                           ordinals,authored,groups)
        migrations = {i:row for spec in specs for i,row in spec['migrations'].items()}
        if len(migrations)!=8:
            raise ValueError('Coupled cheek frame ownership is incomplete')
        for i in migrations:
            if any(abs(e[crease])>1e-8 or e[group] for e in parents[i].link_edges):
                raise ValueError('Cannot move an authored cheek crease')
        selected = {original_faces[i] for spec in specs for i in spec['selected']}
        boundary = {parents[i] for spec in specs for i in spec['boundary']}
        old_vertices = {v for face in selected for v in face.verts}
        old_edges = {e for face in selected for e in face.edges}
        preserved = {bm.edges.get((parents[a],parents[b])) for spec in specs
                     for a,b in zip(spec['boundary'],spec['boundary'][1:]+spec['boundary'][:1])}
        if None in preserved:
            raise ValueError('Missing existing nasal boundary edge')
        edge_snapshot = {e:(float(e[crease]),int(e[group])) for e in preserved}
        if any(abs(e[crease])>1e-8 or e[group] for e in old_edges-preserved):
            raise ValueError('Cannot erase an internal authored nasal crease')
        if any(any(f not in selected for f in v.link_faces) for v in old_vertices-boundary):
            raise ValueError('Nasal interior vertex is owned outside replacement domain')
        if len({f.material_index for f in selected}) != 1:
            raise ValueError('Nasal patch crosses a material boundary')
        for kind in ('int','float','string','float_vector','color','float_color'):
            accessor = getattr(bm.faces.layers,kind,None)
            if accessor is not None:
                for layer in accessor.values():
                    samples = [face[layer] for face in selected]
                    if any(value != samples[0] for value in samples[1:]):
                        raise ValueError('Nasal patch has unsupported varying face provenance')
        for kind in ('int','float','string','float_vector'):
            accessor = getattr(bm.loops.layers,kind,None)
            if accessor is not None and list(accessor.values()):
                raise ValueError('Unsupported nasal corner attribute: '+kind)
        uv_layers = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
        # Include donor faces for any lineage point on a domain edge. Continuity
        # is checked before allocation, including the unchanged outside seam.
        donor_ids = {i for spec in specs for ids,_ in spec['lineage'] for i in ids}
        donor_ids |= {i for spec in specs for i in spec['boundary']}
        donor_ids |= {i for row in migrations.values() for i in row['lineage'][0]}
        donors = {f for i in donor_ids for f in parents[i].link_faces if f[region] == 1}
        old_loops,values = ear_loop_values(donors,{v:i for i,v in enumerate(parents)},uv_layers,colors)
        snapshots = snapshot_nasal_fields(bm,parents,migrations)
        added = []
        for spec in specs:
            allocated = [parents[i] for i in spec['boundary']]
            lineage = [{i:1.} for i in spec['boundary']]
            for point,(ids,shares) in zip(spec['points'],spec['lineage']):
                quantized = np.asarray(point,dtype=np.float32).astype(float)
                depth(float(quantized[0]),float(quantized[2]+.021),interpolated_depth=float(-quantized[1]))
                weights = {int(i):float(w) for i,w in zip(ids,shares) if w>1e-12}
                vertex = bm.verts.new(Vector(point),parents[max(weights,key=weights.get)])
                flow_point_fields(bm,vertex,parents,weights)
                allocated.append(vertex); lineage.append(weights)
            example = original_faces[min(spec['selected'])]
            for quad in spec['quads']:
                face = bm.faces.new([allocated[i] for i in quad],example)
                face.smooth = True
                corner_weights = {allocated[i]:lineage[i] for i in quad}
                for loop in face.loops:
                    assign_ear_loop(loop,corner_weights[loop.vert],old_loops,values,uv_layers,colors)
                added.append(face)
        # Update both old outside corners and new inside corners from immutable
        # donors. Never let one migrated parent contaminate the next's fields.
        for i,row in migrations.items():
            point = row['point']
            depth(float(point[0]),float(point[2]+.021),interpolated_depth=float(-point[1]))
            ids,shares = row['lineage']
            weights = {int(j):float(w) for j,w in zip(ids,shares) if w>1e-12}
            vertex = parents[i]
            if any(f[region]!=1 for f in vertex.link_faces):
                raise ValueError('Coupled cheek node touches a non-skin feature')
            apply_nasal_fields(vertex,snapshots[i])
            vertex.co = Vector(point)
            for face in vertex.link_faces:
                for loop in face.loops:
                    if loop.vert == vertex:
                        assign_ear_loop(loop,weights,old_loops,values,uv_layers,colors)
        for face in selected:
            bm.faces.remove(face)
        for edge in old_edges-preserved:
            if edge.is_valid and not edge.link_faces:
                bm.edges.remove(edge)
        for vertex in old_vertices-boundary:
            if vertex.is_valid and not vertex.link_faces and not vertex.link_edges:
                bm.verts.remove(vertex)
        for edge,expected in edge_snapshot.items():
            if not edge.is_valid or len(edge.link_faces)!=2 or (float(edge[crease]),int(edge[group]))!=expected:
                raise ValueError('Nasal replacement changed a fixed source interface')
        if any(tuple(v.co)!=tuple(migrations[i]['point'] if i in migrations else vertices[i])
               for i,v in enumerate(parents) if v.is_valid):
            raise ValueError('Nasal source replacement moved a retained original point')
        if any(len(f.verts)!=4 or f[region]!=1 for f in added):
            raise ValueError('Nasal source replacement must contain skin quads only')
        if len(bm.verts)-len(bm.edges)+len(bm.faces) != euler_before:
            raise ValueError('Nasal replacement changed source Euler characteristic')
        if sum(len(e.link_faces)==1 for e in bm.edges)!=open_edges_before:
            raise ValueError('Nasal replacement changed open source boundaries')
        if any(len(e.link_faces)!=2 for face in added for e in face.edges):
            raise ValueError('Nasal replacement introduced nonmanifold connectivity')
        bm.normal_update()
        report.update(removed_faces=len(selected),added_quads=len(added),
                      new_sampling_points=sum(len(s['points']) for s in specs),
                      moved_shared_boundary_vertices=len(migrations),
                      euler_preserved=euler_before,open_edges_preserved=open_edges_before)
        bm.to_mesh(data)
        data.update()
        return report
    finally:
        bm.free()
