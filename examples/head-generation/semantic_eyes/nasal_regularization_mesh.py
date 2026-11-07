"""Detached source-only nasal regularization; automatic export remains unchanged.

This is an experimental expanded-region trial, not artistic acceptance. The
original nasal replacement gate is unchanged. Runtime surface and expression
checks must inspect the automatically exported result, not a preferred diagonal.
"""
import numpy as np
from nasal_regularization_solver import propose_nasal_rule
from nasal_regularization_trace import trace_outer_flow
from nasal_joint_contract import nasal_oriented_cycle, nasal_quad_quality


def regularize_nasal_source(data, ordinals, records, joint_preflight=None):
    import bmesh
    from ear_topology import ear_loop_values, assign_ear_loop
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
            raise ValueError('Expanded nasal source ownership is missing')
        regions = [f[region] for f in old_faces]
        traced = [trace_outer_flow(vertices, faces, regions, ordinals,
                                  item['core'], item['anchor']) for item in records[:5]]
        paths = [item['full_path'] for item in traced]
        result, proposed, _, _, _ = propose_nasal_rule(vertices, faces, regions, ordinals, paths)
        # Solver uses X, height, positive-front internally; data uses Blender axes.
        proposed = proposed[:, [0, 2, 1]].copy()
        proposed[:, 1] *= -1
        proposed = proposed.astype(np.float32).astype(float)
        moved = set(map(int, result['moved_vertex_ids']))
        changed = {int(i): q for i, q in result['new_faces'].items()}
        affected = {int(i) for i in result['affected_faces']}
        final_intermediate = [changed.get(i,q) for i,q in enumerate(faces)]
        checked = set(affected)
        if joint_preflight is not None:
            proof = joint_preflight(proposed, final_intermediate, regions, paths, result)
            removed_cycles = {tuple(q) for q in proof['removed_cycles']}
            checked = {i for i in affected if nasal_oriented_cycle(final_intermediate[i]) not in removed_cycles}
            result['joint_final_domain'] = proof
        stored_quality = (nasal_quad_quality(proposed,[final_intermediate[i] for i in sorted(checked)])
                          if checked else dict(min_angle=None,max_angle=None,invalid_faces=[],checked_faces=0))
        maximum_move = float(np.linalg.norm(proposed-vertices,axis=1).max())
        if stored_quality['invalid_faces'] or maximum_move>.00250001:
            raise ValueError('Float32 expanded nasal geometry failed')
        result['stored_geometry'] = dict(min_angle=stored_quality['min_angle'],
                                        max_angle=stored_quality['max_angle'],max_move_m=maximum_move,
                                        checked_faces=stored_quality['checked_faces'])
        lineage = {int(i): {int(k): float(w) for k, w in zip(row['donors'], row['weights'])
                            if w > 1e-12} for i, row in result['lineage'].items()}
        quality = result['candidate_quality']
        if (not result['outer_boundary_fixed'] or not result['protected_vertices_unchanged']
                or (joint_preflight is None and (quality['min_angle'] < 10 or quality['max_angle'] > 170
                    or quality['nonconvex_front_faces'])) or result['maximum_move_m'] > .00250001):
            raise ValueError('Expanded nasal trial violated geometric bounds')
        for weights in lineage.values():
            if not weights or abs(sum(weights.values())-1) > 1e-8 or min(weights.values()) < 0:
                raise ValueError('Invalid expanded nasal donor weights')
        euler = len(bm.verts)-len(bm.edges)+len(bm.faces)
        open_edges = {tuple(sorted(v.index for v in e.verts)) for e in bm.edges if len(e.link_faces)==1}
        edges = {tuple(sorted(v.index for v in e.verts)): (float(e[crease]), int(e[group])) for e in bm.edges}
        # Freeze all donor fields before moving any vertex, avoiding order dependence.
        fields = []
        for kind in ('float', 'float_vector', 'color', 'float_color', 'shape', 'int', 'string'):
            accessor = getattr(bm.verts.layers, kind, None)
            for layer in accessor.values() if accessor is not None else []:
                values = [v[layer] if kind in ('float','int','string') else tuple(v[layer]) for v in parents]
                fields.append((kind, layer, values))
        deforms = [(layer, [dict(v[layer]) for v in parents]) for layer in bm.verts.layers.deform.values()]
        uv = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
        for kind in ('int','float','string','float_vector'):
            accessor = getattr(bm.loops.layers, kind, None)
            if accessor is not None and list(accessor.values()):
                raise ValueError('Unsupported expanded nasal corner field')
        donor_ids = {i for row in lineage.values() for i in row}
        donor_ids |= {i for fi in affected for i in faces[fi]}
        donors = {f for i in donor_ids for f in parents[i].link_faces if f[region]==1}
        old_loops, values = ear_loop_values(donors, {v:i for i,v in enumerate(parents)}, uv, colors)
        for i in moved:
            weights = lineage[i]
            for kind, layer, snapshots in fields:
                first = snapshots[next(iter(weights))]
                if kind in ('int','string'):
                    if any(snapshots[k] != first for k in weights):
                        raise ValueError('Expanded nasal move crosses discrete ownership')
                    parents[i][layer] = first
                elif kind == 'float':
                    parents[i][layer] = sum(w*snapshots[k] for k,w in weights.items())
                else:
                    parents[i][layer] = tuple(sum(w*snapshots[k][c] for k,w in weights.items()) for c in range(len(first)))
            for layer, snapshots in deforms:
                parents[i][layer].clear()
                keys = set().union(*(snapshots[k].keys() for k in weights))
                for key in keys:
                    parents[i][layer][key] = sum(w*snapshots[k].get(key,0) for k,w in weights.items())
            parents[i].co = proposed[i]
        replacement = {}
        for fi, quad in changed.items():
            old = old_faces[fi]
            if old[region] != 1 or len(old.verts) != 4:
                raise ValueError('Expanded nasal edit crosses skin ownership')
            new = bm.faces.new([parents[i] for i in quad], old)
            new.smooth = old.smooth
            replacement[fi] = new
        old_edges = {e for fi in changed for e in old_faces[fi].edges}
        for fi in changed:
            bm.faces.remove(old_faces[fi])
        for edge in old_edges:
            if edge.is_valid and not edge.link_faces:
                if abs(edge[crease]) > 1e-8 or edge[group]:
                    raise ValueError('Expanded nasal edit removes a protected crease')
                bm.edges.remove(edge)
        for fi in affected:
            face = replacement.get(fi, old_faces[fi])
            for loop in face.loops:
                i = loop.vert.index
                assign_ear_loop(loop, lineage.get(i,{i:1.}), old_loops, values, uv, colors)
        bm.verts.index_update()
        current_edges = {tuple(sorted(v.index for v in e.verts)):e for e in bm.edges}
        for key, edge in current_edges.items():
            if key in edges:
                if (float(edge[crease]),int(edge[group])) != edges[key]:
                    raise ValueError('Expanded nasal edit changed retained edge fields')
            else:
                edge[crease], edge[group] = 0., 0
        if len(bm.verts)!=len(parents) or len(bm.verts)-len(bm.edges)+len(bm.faces)!=euler:
            raise ValueError('Expanded nasal edit changed vertex count or Euler characteristic')
        if {k for k,e in current_edges.items() if len(e.link_faces)==1} != open_edges:
            raise ValueError('Expanded nasal edit changed an open boundary')
        if any(len(e.link_faces)!=2 for fi in affected for e in replacement.get(fi,old_faces[fi]).edges):
            raise ValueError('Expanded nasal edit created nonmanifold connectivity')
        if any(tuple(v.co)!=tuple(vertices[i]) for i,v in enumerate(parents) if i not in moved):
            raise ValueError('Expanded nasal edit moved an unrelated vertex')
        bm.normal_update()
        bm.to_mesh(data)
        data.update()
        result.update(source_paths=paths, traced_source_paths=traced, stage='expanded-nasal-native-trial',
                      runtime_acceptance='pending actual automatic-export and expression verification')
        return result
    finally:
        bm.free()
