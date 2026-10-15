"""Native read-only UV projection diagnosis; reports rejected data without baking."""
import numpy as np


def diagnose_triangles(triangles, resolution=256):
    """Each record includes mesh triangle IDs, UVs, positions and corner normals."""
    samples = {}
    degenerates, conflicts = [], []
    overlap_count = conflict_count = 0
    for record in triangles:
        uv = np.asarray(record['uv'], dtype=float).copy()
        uv[:, 1] = 1 - uv[:, 1]
        positions = np.asarray(record['positions_blender'], dtype=float)
        normals = np.asarray(record['normals_godot'], dtype=float)
        a, b, c = uv
        det = (b-a)[0] * (c-a)[1] - (b-a)[1] * (c-a)[0]
        if abs(det) < 1e-14:
            degenerates.append({**record, 'uv_determinant': float(det),
                                'geometric_twice_area': float(np.linalg.norm(np.cross(positions[1]-positions[0], positions[2]-positions[0])))})
            continue
        lo = np.maximum(0, np.ceil(uv.min(axis=0) * resolution - .5).astype(int))
        hi = np.minimum(resolution-1, np.floor(uv.max(axis=0) * resolution - .5).astype(int))
        for y in range(lo[1], hi[1]+1):
            for x in range(lo[0], hi[0]+1):
                q = np.array([(x+.5)/resolution, (y+.5)/resolution])-a
                beta = (q[0]*(c-a)[1]-q[1]*(c-a)[0])/det
                gamma = ((b-a)[0]*q[1]-(b-a)[1]*q[0])/det
                weights = np.array([1-beta-gamma, beta, gamma])
                if weights.min() < -1e-9:
                    continue
                n = weights @ normals
                length = np.linalg.norm(n)
                if length > 1e-15:
                    n /= length
                position = weights @ positions
                hit = {'triangle': record, 'pixel': [x, y], 'barycentric': weights.tolist(),
                       'position_blender': position.tolist(), 'normal_godot': n.tolist()}
                key = (x, y)
                if key in samples:
                    overlap_count += 1
                    previous = samples[key]
                    error = float(np.max(np.abs(n - previous['normal_godot'])))
                    if error > 1e-4:
                        conflict_count += 1
                        if len(conflicts) < 128:
                            before = np.asarray(previous['position_blender'])
                            conflicts.append({'first': previous, 'second': hit, 'normal_max_component_difference': error,
                                              'position_separation_m': float(np.linalg.norm(position-before)),
                                              'forward_depth_difference_m': float(-(position[1]-before[1]))})
                else:
                    samples[key] = hit
    return {'resolution': resolution, 'triangles': len(triangles), 'covered_pixels': len(samples),
            'covered_fraction': len(samples)/resolution**2, 'degenerate_uv_triangles': len(degenerates),
            'overlap_samples': overlap_count, 'conflicting_overlap_samples': conflict_count,
            'degenerate_records': degenerates, 'conflict_records': conflicts,
            'conflict_record_limit': 128, 'acceptance': 'diagnostic only; no gate relaxation or atlas output'}


def diagnose_blend(path):
    import bpy
    bpy.ops.wm.open_mainfile(filepath=str(path), load_ui=False, use_scripts=False)
    obj = bpy.data.objects.get('Face')
    assert obj and obj.type == 'MESH'
    data = obj.data
    if data.shape_keys:
        for key in data.shape_keys.key_blocks:
            key.value = 0
    data.calc_loop_triangles()
    uv, regions = data.uv_layers['UV2'], data.attributes['NormalRegion']
    records = []
    for index, tri in enumerate(data.loop_triangles):
        if regions.data[tri.polygon_index].value != 1:
            continue
        records.append({'triangle_index': index, 'polygon_index': tri.polygon_index,
                        'vertex_indices': list(tri.vertices), 'loop_indices': list(tri.loops),
                        'polygon_normal_blender': list(data.polygons[tri.polygon_index].normal),
                        'positions_blender': [list(data.vertices[v].co) for v in tri.vertices],
                        'uv': [list(uv.data[i].uv) for i in tri.loops],
                        'normals_godot': [[data.corner_normals[i].vector.x, data.corner_normals[i].vector.z,
                                           -data.corner_normals[i].vector.y] for i in tri.loops]})
    result = diagnose_triangles(records)
    result['matrix_world'] = [list(row) for row in obj.matrix_world]
    return result
