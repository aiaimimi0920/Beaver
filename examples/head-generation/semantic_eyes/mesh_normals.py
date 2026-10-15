"""UV2 raster of actual neutral front-shell corner normals, plus explicit hole fill."""
import numpy as np


def raster_normals(triangles, resolution=256):
    field = np.zeros((resolution, resolution, 3), dtype=np.float64)
    counts = np.zeros((resolution, resolution), dtype=np.int32)
    degenerate = overlaps = conflicting = 0
    for uv, normals in triangles:
        uv, normals = np.asarray(uv, dtype=float), np.asarray(normals, dtype=float)
        if uv.shape != (3, 2) or normals.shape != (3, 3) or not np.isfinite(uv).all() or not np.isfinite(normals).all():
            raise ValueError('Invalid normal triangle')
        # Blender UV v is bottom-origin; baker rows are top-origin.
        points = uv.copy()
        points[:, 1] = 1 - points[:, 1]
        a, b, c = points
        determinant = (b - a)[0] * (c - a)[1] - (b - a)[1] * (c - a)[0]
        if abs(determinant) < 1e-14:
            degenerate += 1
            continue
        lo = np.maximum(0, np.ceil(points.min(axis=0) * resolution - .5).astype(int))
        hi = np.minimum(resolution - 1, np.floor(points.max(axis=0) * resolution - .5).astype(int))
        if np.any(lo > hi):
            continue
        x, y = np.meshgrid(np.arange(lo[0], hi[0] + 1), np.arange(lo[1], hi[1] + 1))
        q = np.stack(((x + .5) / resolution, (y + .5) / resolution), axis=-1) - a
        beta = (q[..., 0] * (c - a)[1] - q[..., 1] * (c - a)[0]) / determinant
        gamma = ((b - a)[0] * q[..., 1] - (b - a)[1] * q[..., 0]) / determinant
        alpha = 1 - beta - gamma
        inside = (alpha >= -1e-9) & (beta >= -1e-9) & (gamma >= -1e-9)
        interpolated = alpha[..., None] * normals[0] + beta[..., None] * normals[1] + gamma[..., None] * normals[2]
        prior_count = counts[y[inside], x[inside]]
        overlap = prior_count > 0
        overlaps += int(overlap.sum())
        if overlap.any():
            prior = field[y[inside][overlap], x[inside][overlap]] / prior_count[overlap, None]
            incoming = interpolated[inside][overlap]
            prior /= np.maximum(np.linalg.norm(prior, axis=-1, keepdims=True), 1e-15)
            incoming = incoming / np.maximum(np.linalg.norm(incoming, axis=-1, keepdims=True), 1e-15)
            conflicting += int((np.max(np.abs(prior - incoming), axis=-1) > 1e-4).sum())
        field[y[inside], x[inside]] += interpolated[inside]
        counts[y[inside], x[inside]] += 1
    covered = counts > 0
    if not covered.any():
        raise ValueError('No front-shell UV coverage')
    diagnostics = {'degenerate_uv_triangles': degenerate, 'overlap_samples': overlaps,
                   'conflicting_overlap_samples': conflicting, 'covered_fraction': float(covered.mean())}
    if degenerate or conflicting or covered.mean() < .10:
        raise ValueError('Unaccepted normal raster coverage/UV diagnostics: ' + str(diagnostics))
    field[covered] /= counts[covered, None]
    initial = int(covered.sum())
    # Explicit continuation through eye/mouth openings and outside the face;
    # these pixels are extrapolated, never claimed to be measured mesh samples.
    rounds = 0
    while not covered.all():
        sums = np.zeros_like(field)
        neighbors = np.zeros_like(counts)
        for dst, src in [((slice(1, None), slice(None)), (slice(None, -1), slice(None))),
                         ((slice(None, -1), slice(None)), (slice(1, None), slice(None))),
                         ((slice(None), slice(1, None)), (slice(None), slice(None, -1))),
                         ((slice(None), slice(None, -1)), (slice(None), slice(1, None)))]:
            sums[dst] += field[src] * covered[src][..., None]
            neighbors[dst] += covered[src]
        new = ~covered & (neighbors > 0)
        if not new.any():
            raise ValueError('Disconnected fill frontier')
        field[new] = sums[new] / neighbors[new, None]
        covered[new] = True
        rounds += 1
        if rounds > int(np.ceil(resolution * .8)):
            raise ValueError('Unaccepted normal raster extrapolation distance: ' + str(rounds))
    lengths = np.linalg.norm(field, axis=-1)
    if not np.isfinite(lengths).all() or (lengths < 1e-8).any():
        raise ValueError('Invalid or cancelling normal field')
    field /= lengths[..., None]
    return field, {**diagnostics, 'minimum_covered_fraction': .10, 'maximum_fill_rounds': int(np.ceil(resolution * .8)), 'mesh_covered_pixels': initial, 'extrapolated_pixels': resolution ** 2 - initial,
                   'fill_rounds': rounds, 'policy': 'front-shell actual corner normals; four-neighbor uncovered-pixel continuation'}


def is_front_facing(positions):
    """The planar UV2 chart views Blender -Y; return/back faces have no chart area."""
    points = np.asarray(positions, dtype=float)
    if points.shape != (3, 3) or not np.isfinite(points).all():
        raise ValueError('Invalid geometric triangle')
    normal = np.cross(points[1] - points[0], points[2] - points[0])
    length = float(np.linalg.norm(normal))
    if length < 1e-14:
        raise ValueError('Degenerate geometric triangle')
    return normal[1] / length < -1e-10


def from_runtime_blend(path, resolution=256):
    import bpy
    bpy.ops.wm.open_mainfile(filepath=str(path), load_ui=False, use_scripts=False)
    obj = bpy.data.objects.get('Face')
    assert obj is not None and obj.type == 'MESH', 'Missing runtime Face'
    assert all(abs(v - wanted) < 1e-9 for row, expected in zip(obj.matrix_world, np.eye(4)) for v, wanted in zip(row, expected)), 'Expected identity baked Face transform'
    if obj.data.shape_keys:
        for key in obj.data.shape_keys.key_blocks:
            key.value = 0
    data = obj.data
    regions = data.attributes.get('NormalRegion')
    uv = data.uv_layers.get('UV2')
    assert regions is not None and regions.domain == 'FACE' and uv is not None
    data.calc_loop_triangles()
    triangles, excluded = [], []
    for triangle_index, tri in enumerate(data.loop_triangles):
        if regions.data[tri.polygon_index].value != 1:
            continue
        positions = [tuple(data.vertices[i].co) for i in tri.vertices]
        if not is_front_facing(positions):
            excluded.append(triangle_index)
            continue
        coords, normals = [], []
        for index in tri.loops:
            coords.append(tuple(uv.data[index].uv))
            n = data.corner_normals[index].vector
            normals.append((n.x, n.z, -n.y))
        triangles.append((coords, normals))
    field, report = raster_normals(triangles, resolution)
    report['mesh_triangles'] = len(triangles)
    report['excluded_back_or_edge_facing_triangles'] = len(excluded)
    report['excluded_triangle_indices'] = excluded
    report['chart_selection'] = 'NormalRegion1 AND geometric triangle normal toward Blender -Y; back/edge-facing return faces excluded before unchanged raster gates'
    report['coordinate_system'] = 'Godot x/y/z from Blender x/z/-y'
    return field, report
