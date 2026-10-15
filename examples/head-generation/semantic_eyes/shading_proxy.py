"""Geometry-fitted art-directed shading directions, deliberately not mesh normals."""
import hashlib
from pathlib import Path
import numpy as np

POLICY = 'geometry_fitted_smooth_convex_proxy_v1'


def fit_proxy(uv, positions):
    """Fit the authored planar UV2 map and neutral front-shell silhouette bounds.

    Positions are Blender XYZ. Depth Y is intentionally excluded: eyelid, nose,
    lip and chin relief must not introduce disconnected directional shadow islands.
    Clamped UV boundary samples contribute bounds but not the affine-map fit.
    """
    uv, positions = np.asarray(uv, dtype=float), np.asarray(positions, dtype=float)
    if uv.ndim != 2 or uv.shape[1:] != (2,) or positions.shape != (len(uv), 3):
        raise ValueError('Expected paired UV2 and neutral XYZ samples')
    if len(uv) < 8 or not np.isfinite(uv).all() or not np.isfinite(positions).all():
        raise ValueError('Insufficient or nonfinite proxy samples')
    if (uv < -1e-6).any() or (uv > 1 + 1e-6).any():
        raise ValueError('Expected bounded authored UV2')
    interior = ((uv > 1e-6) & (uv < 1 - 1e-6)).all(axis=1)
    design = np.column_stack((uv[interior], np.ones(int(interior.sum()))))
    if len(design) < 8 or np.linalg.matrix_rank(design) != 3:
        raise ValueError('Insufficient planar UV2 fit support')
    xz = positions[:, (0, 2)]
    if np.min(np.ptp(uv[interior], axis=0)) < .2 or np.linalg.cond(design) > 100:
        raise ValueError('Insufficient or ill-conditioned UV2 coverage')
    # Final local nose/ear refinements may move X/Z after UV2 assignment.
    # Recover the dominant authored map, never an unrelated hardcoded fallback.
    selected = np.ones(len(design), dtype=bool)
    for _ in range(12):
        coefficients, _, rank, _ = np.linalg.lstsq(design[selected], xz[interior][selected], rcond=None)
        if rank != 3:
            raise ValueError('Degenerate trimmed UV2 fit')
        errors = design @ coefficients - xz[interior]
        residual = np.max(np.abs(errors), axis=1)
        order = np.argsort(residual, kind='stable')
        selected[:] = False
        selected[order[:max(8, int(np.ceil(.7 * len(design))))]] = True
    inliers = residual <= 2e-6
    if inliers.mean() < .7 or np.min(np.ptp(uv[interior][inliers], axis=0)) < .2:
        raise ValueError('UV2 lacks a supported dominant planar face projection')
    if np.linalg.cond(design[inliers]) > 100:
        raise ValueError('Ill-conditioned accepted UV2 fit')
    coefficients, _, _, _ = np.linalg.lstsq(design[inliers], xz[interior][inliers], rcond=None)
    errors = design @ coefficients - xz[interior]
    maximum = float(np.max(np.abs(errors[inliers])))
    if maximum > 2e-6:
        raise ValueError('Accepted UV2 fit exceeds residual bound')
    # Explicit orientation contract shared with the Godot UV2/top-origin baker.
    if coefficients[0, 0] <= 0 or coefficients[1, 1] <= 0 or max(abs(coefficients[0, 1]), abs(coefficients[1, 0])) > 2e-6:
        raise ValueError('Unsupported flipped or rotated UV2 axes')
    low, high = xz.min(axis=0), xz.max(axis=0)
    spans = high - low
    if np.any(spans < .02) or np.any(spans > 1):
        raise ValueError('Implausible meter-scale front-shell bounds')
    return {'policy': POLICY, 'uv2_to_blender_xz': coefficients.tolist(),
            'bounds_blender_xz_m': [low.tolist(), high.tolist()],
            'center_x_m': float((low[0] + high[0]) / 2),
            'half_width_m': float(spans[0] / 2), 'height_m': float(spans[1]),
            'vertical_center_m': float(low[1] + .52 * spans[1]),
            'fit_samples': int(inliers.sum()), 'interior_samples': int(interior.sum()),
            'local_refinement_outliers': int((~inliers).sum()), 'bounds_samples': len(uv),
            'fit_inlier_fraction': float(inliers.mean()),
            'fit_condition_number': float(np.linalg.cond(design[inliers])),
            'all_sample_max_component_error_m': float(np.max(np.abs(errors))),
            'all_sample_residual_percentiles_m': dict(zip(('median', 'p95', 'p99', 'max'),
                [float(v) for v in np.percentile(np.max(np.abs(errors), axis=1), [50, 95, 99, 100])])),
            'sample_centroid_blender_xz_m': xz.mean(axis=0).tolist(),
            'sample_extent_blender_xz_m': spans.tolist(),
            'interior_uv_bounds': [uv[interior].min(axis=0).tolist(), uv[interior].max(axis=0).tolist()],
            'inlier_uv_bounds': [uv[interior][inliers].min(axis=0).tolist(), uv[interior][inliers].max(axis=0).tolist()],
            'excluded_refinement_residual_bounds_xz_m': ([errors[~inliers].min(axis=0).tolist(), errors[~inliers].max(axis=0).tolist()] if (~inliers).any() else None),
            'excluded_refinement_centroid_blender_xz_m': (xz[interior][~inliers].mean(axis=0).tolist() if (~inliers).any() else None),
            'fit_max_component_error_m': maximum,
            'fit_rms_component_error_m': float(np.sqrt(np.mean(errors[inliers] ** 2))),
            'lateral_gain': .83, 'vertical_gain': .62, 'vertical_slope': 2.4,
            'limits': 'Art-directed shading directions, not physical mesh normals. Uses global front-shell silhouette bounds and a dominant planar UV2 approximation; excluded local refinement residuals are reported, not claimed exact; intentionally excludes eye/nose/lip/depth relief, cast occlusion and expression deformation. Visual acceptance pending.'}


def proxy_normals(fit, resolution=256):
    if isinstance(resolution, bool) or not isinstance(resolution, int) or resolution < 2:
        raise ValueError('Expected integer raster resolution >= 2')
    u = (np.arange(resolution) + .5) / resolution
    v = 1 - (np.arange(resolution) + .5) / resolution
    uu, vv = np.meshgrid(u, v)
    xz = np.stack((uu, vv, np.ones_like(uu)), axis=-1) @ np.asarray(fit['uv2_to_blender_xz'])
    side = np.clip((xz[..., 0] - fit['center_x_m']) / fit['half_width_m'], -.985, .985)
    height = (xz[..., 1] - fit['vertical_center_m']) / fit['height_m']
    nx = side * fit['lateral_gain']
    ny = np.tanh(height * fit['vertical_slope']) * fit['vertical_gain']
    nz = np.sqrt(np.maximum(.08, 1 - np.minimum(.92, nx * nx + ny * ny)))
    field = np.stack((nx, ny, nz), axis=-1)
    field /= np.linalg.norm(field, axis=-1, keepdims=True)
    if not np.isfinite(field).all():
        raise ValueError('Invalid shading-proxy direction field')
    return field


def from_runtime_blend(path, resolution=256):
    import bpy
    path = Path(path)
    source_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(path), load_ui=False, use_scripts=False)
    obj = bpy.data.objects.get('Face')
    assert obj is not None and obj.type == 'MESH', 'Missing runtime Face'
    assert np.allclose(np.asarray(obj.matrix_world), np.eye(4), atol=1e-9, rtol=0), 'Expected identity baked Face transform'
    data = obj.data
    assert data.shape_keys is not None and 'Basis' in data.shape_keys.key_blocks, 'Expected neutral Basis'
    for key in data.shape_keys.key_blocks:
        key.value = 0
    basis = data.shape_keys.key_blocks['Basis']
    assert all((v.co - b.co).length < 1e-7 for v, b in zip(data.vertices, basis.data)), 'Mesh coordinates differ from neutral Basis'
    regions, layer = data.attributes.get('NormalRegion'), data.uv_layers.get('UV2')
    assert regions is not None and regions.domain == 'FACE' and layer is not None
    # Select the semantic front shell; silhouette fit does not differentiate
    # polygon normals, so folded/back-facing returns cannot create shadow islands.
    samples = {}
    faces = 0
    for poly in data.polygons:
        if regions.data[poly.index].value != 1:
            continue
        faces += 1
        for index in poly.loop_indices:
            vertex_id = data.loops[index].vertex_index
            uv = tuple(layer.data[index].uv)
            samples[(vertex_id, uv)] = (uv, tuple(basis.data[vertex_id].co))
    uv, points = zip(*samples.values()) if samples else ([], [])
    fit = fit_proxy(uv, points)
    fit.update(source_runtime_blend_sha256=source_hash, selected_region=1,
               selected_faces=faces, resolution=resolution,
               coordinate_system='Godot shading x/y/z; planar Blender x/z and positive frontal Godot z; raster rows top-origin')
    return proxy_normals(fit, resolution), fit
