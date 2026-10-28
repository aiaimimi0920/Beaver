"""Rebuild the inner nasal band atomically from current semantic source paths."""
import numpy as np
from nasal_inner_rule import propose_inner_nasal
from nasal_source_mesh import claim_nasal_source_face


def rebuild_inner_nasal_source(data, ordinals, paths):
    import bmesh
    from mathutils import Vector
    from ear_topology import flow_point_fields, ear_loop_values, assign_ear_loop
    bm = bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bm.verts.index_update()
        bm.faces.index_update()
        parents, old_faces = list(bm.verts), list(bm.faces)
        vertices = np.asarray([tuple(v.co) for v in parents])
        faces = [tuple(v.index for v in f.verts) for f in old_faces]
        region = bm.faces.layers.int.get('NormalRegion')
        crease = bm.edges.layers.float.get('crease_edge')
        group = bm.edges.layers.int.get('FacialCreaseGroup')
        if region is None or crease is None or group is None:
            raise ValueError('Inner nasal source ownership missing')
        spec, proposed, _ = propose_inner_nasal(vertices, faces,
                                               [f[region] for f in old_faces],ordinals,paths)
        stored = proposed.astype(np.float32).astype(float)[np.asarray(spec['new_quads'])]
        a,b = np.roll(stored,1,axis=1)-stored,np.roll(stored,-1,axis=1)-stored
        norm = np.linalg.norm(a,axis=2)*np.linalg.norm(b,axis=2)
        if np.any(norm<1e-12):
            raise ValueError('Inner nasal stored edge collapsed')
        angles = np.degrees(np.arccos(np.clip(np.sum(a*b,axis=2)/norm,-1,1)))
        projected = np.cross(a,b)[:,:,1]
        if angles.min()<10 or angles.max()>170 or np.any(projected.min(axis=1)*projected.max(axis=1)<=0):
            raise ValueError('Inner nasal stored corner angle invalid')
        spec['stored_angles'] = [float(angles.min()),float(angles.max())]
        selected = {old_faces[i] for i in spec['removed_faces']}
        deleted = {parents[i] for i in spec['removed_vertices']}
        if any(any(f not in selected for f in v.link_faces) for v in deleted):
            raise ValueError('Inner nasal removal crosses source ownership')
        if len({f.material_index for f in selected}) != 1:
            raise ValueError('Inner nasal replacement crosses materials')
        for kind in ('int','float','string','float_vector','color','float_color'):
            accessor = getattr(bm.faces.layers,kind,None)
            for layer in accessor.values() if accessor is not None else []:
                samples = [face[layer] for face in selected]
                if any(value != samples[0] for value in samples[1:]):
                    raise ValueError('Inner nasal replacement crosses discrete face provenance')
        for kind in ('int','float','string','float_vector'):
            accessor = getattr(bm.loops.layers,kind,None)
            if accessor is not None and list(accessor.values()):
                raise ValueError('Unsupported inner nasal corner field')
        uv = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
        old_loops, values = ear_loop_values(selected,{v:i for i,v in enumerate(parents)},uv,colors)
        edges_before = {tuple(sorted(v.index for v in e.verts)):(float(e[crease]),int(e[group])) for e in bm.edges}
        open_before = {key for key in edges_before if len(bm.edges.get(tuple(parents[i] for i in key)).link_faces)==1}
        euler = len(bm.verts)-len(bm.edges)+len(bm.faces)
        allocated = list(parents)
        lineage = {i:{i:1.} for i in range(len(parents))}
        for i in spec['new_point_ids']:
            if i != len(allocated):
                raise ValueError('Inner nasal point order is not contiguous')
            row = spec['new_point_lineage'][i]
            weights = {int(k):float(w) for k,w in zip(row['donors'],row['weights']) if w>1e-12}
            if not weights or min(weights.values())<0 or abs(sum(weights.values())-1)>1e-8:
                raise ValueError('Invalid inner nasal source lineage')
            point = np.asarray(proposed[i],dtype=np.float32).astype(float)
            vertex = bm.verts.new(Vector(point),parents[max(weights,key=weights.get)])
            flow_point_fields(bm,vertex,parents,weights)
            allocated.append(vertex)
            lineage[i] = weights
        old_edges = {e for f in selected for e in f.edges}
        reused, added = set(), []
        example = old_faces[min(spec['removed_faces'])]
        for quad in spec['new_quads']:
            ordered = [allocated[i] for i in quad]
            face = bm.faces.get(ordered)
            if face is None:
                face = bm.faces.new(ordered,example)
            else:
                claim_nasal_source_face(face,ordered,selected,reused)
            face.smooth = True
            local = {allocated[i]:lineage[i] for i in quad}
            for loop in face.loops:
                assign_ear_loop(loop,local[loop.vert],old_loops,values,uv,colors)
            added.append(face)
        for face in selected-reused:
            bm.faces.remove(face)
        for edge in old_edges:
            if edge.is_valid and not edge.link_faces:
                if abs(edge[crease])>1e-8 or edge[group]:
                    raise ValueError('Inner nasal replacement removes authored crease')
                bm.edges.remove(edge)
        for vertex in deleted:
            if vertex.link_faces or vertex.link_edges:
                raise ValueError('Inner nasal point remains owned by another source element')
            bm.verts.remove(vertex)
        if any(tuple(v.co)!=tuple(vertices[i]) for i,v in enumerate(parents) if v.is_valid):
            raise ValueError('Inner nasal replacement moved a retained point')
        if len(bm.verts)-len(bm.edges)+len(bm.faces)!=euler:
            raise ValueError('Inner nasal replacement changed Euler characteristic')
        original_index = {v:i for i,v in enumerate(parents) if v.is_valid}
        for edge in bm.edges:
            if all(v in original_index for v in edge.verts):
                key = tuple(sorted(original_index[v] for v in edge.verts))
                if key in edges_before and (float(edge[crease]),int(edge[group]))!=edges_before[key]:
                    raise ValueError('Inner nasal retained edge fields changed')
        if {tuple(sorted(original_index[v] for v in e.verts)) for e in bm.edges if len(e.link_faces)==1}!=open_before:
            raise ValueError('Inner nasal replacement changed open boundaries')
        if any(len(f.verts)!=4 or f[region]!=1 or any(len(e.link_faces)!=2 for e in f.edges) for f in added):
            raise ValueError('Inner nasal replacement is not conforming skin quads')
        bm.verts.index_update()
        spec['source_point_mapping'] = {i:v.index for i,v in enumerate(allocated) if v.is_valid}
        spec['final_full_paths'] = [[allocated[i].index for i in p] for p in spec['updated_full_paths']]
        spec.update(added_quads=len(added),removed_face_count=len(selected),reused_faces=len(reused),
                    native_stage='inner-nasal-source-trial',acceptance='pending native audit and visual review')
        bm.normal_update()
        bm.to_mesh(data)
        data.update()
        return spec
    finally:
        bm.free()
