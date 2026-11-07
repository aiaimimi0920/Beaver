"""Explicit bounded author-shape change, separate from surface-preserving retile."""
import numpy as np
from scipy.interpolate import RBFInterpolator
from forehead_flow_rule import forehead_chart
from forehead_star_topology import forehead_edge_counts
from nasal_source_domain import current_source_features


def regularize_forehead_author_surface(vertices, faces, regions, ordinals):
    old = np.asarray(vertices, np.float32).astype(float)
    features = current_source_features(old, faces, regions, ordinals)
    chart = forehead_chart(old)
    radius = np.median(np.linalg.norm(old[:, :2], axis=1))
    radial = np.linalg.norm(old[:, :2], axis=1)
    theta = chart[:, 0]/radius
    memberships = [set() for _ in old]
    for q, owner in zip(faces, regions):
        for i in q:
            memberships[i].add(int(owner))
    skin = {i for q, owner in zip(faces, regions) if owner == 1 and len(q) == 4 for i in q}
    ids = sorted(i for i in skin if old[i, 0] > 1e-8 and .35 < theta[i] < 1.8
                 and 1.615 < old[i, 2] < 1.755)
    if len(ids) < 20:
        raise ValueError('Insufficient raw13 forehead samples')
    coordinates = np.c_[chart[:, 0]*1000, (old[:, 2]-1.68)*1000]
    fit = RBFInterpolator(coordinates[ids], radial[ids]*1000,
                          kernel='thin_plate_spline', smoothing=1000., degree=1)
    def smooth(values):
        values = np.clip(values, 0, 1)
        return values*values*(3-2*values)
    mask = (smooth((theta-.45)/.25)*smooth((1.7-theta)/.25)
            *smooth((old[:, 2]-1.625)/.018)*smooth((1.735-old[:, 2])/.025))
    boundary = {i for edge, count in forehead_edge_counts(faces).items() if count == 1 for i in edge}
    protected = set(features['protected']) | boundary
    mask[list(protected)] = 0
    mask[sorted(set(range(len(old)))-skin)] = 0
    displacement = .002*np.tanh((fit(coordinates)/1000-radial)/.002)*mask
    new = old.copy()
    new[:, :2] *= ((radial+displacement)/np.maximum(radial, 1e-9))[:, None]
    new[old[:, 0] < 0] = old[old[:, 0] < 0]
    new = np.asarray(new, np.float32).astype(float)
    for i in np.flatnonzero(np.linalg.norm(new-old, axis=1) > 1e-9):
        if old[i, 0] <= 0:
            continue
        candidates = np.flatnonzero(np.linalg.norm(old-old[i]*[-1, 1, 1], axis=1) < 1e-7)
        candidates = [j for j in candidates if memberships[j] == memberships[i]]
        if len(candidates) != 1:
            raise ValueError('Ambiguous author-surface mirror')
        new[candidates[0]] = new[i]*[-1, 1, 1]
    moved = np.flatnonzero(np.linalg.norm(new-old, axis=1) > 1e-9)
    maximum = float(np.linalg.norm(new-old, axis=1).max()*1000)
    if maximum > 2.001 or any(memberships[i] != {1} for i in moved):
        raise ValueError('Author-shape scope or 2 mm displacement cap exceeded')
    if not np.array_equal(new[sorted(protected)], old[sorted(protected)]):
        raise ValueError('Author shape moved a functional boundary')
    return new, {'changed_vertices': moved.tolist(), 'maximum_displacement_mm': maximum,
                 'displacement_cap_mm': 2., 'protected_vertices': sorted(protected),
                 'scope': 'Intentional raw13 author-shape regularization; this is not a '
                          'claim of preserving the original author surface within 1 mm'}
