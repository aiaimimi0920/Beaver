"""Pure initialization aids, not a general geometry-restoration algorithm."""
import copy
import numpy as np
from forehead_pattern_catalogue import forehead_pattern_edge

def forehead_pattern_field_capacity(current_delta, frozen_delta, budget_mm=0.5):
    if budget_mm <= 0:
        raise ValueError('Budget must be positive')
    current = np.asarray(current_delta, float)
    frozen = np.asarray(frozen_delta, float)
    if current.shape != frozen.shape:
        raise ValueError('Frozen/current field dimensions differ')
    drift = np.linalg.norm(current - frozen, axis=-1) * 1000
    return {'drift_mm': drift, 'headroom_mm': budget_mm - drift}

def forehead_pattern_support_halo(faces, seeds, movable, held, layers=1):
    if layers < 0:
        raise ValueError('Negative halo depth')
    active = set(seeds)
    for _ in range(layers):
        previous = set(active)
        active.update((v for q in faces if previous.intersection(q) for v in q))
    return sorted(active.intersection(movable) - set(held))

def forehead_pattern_canonical_birth_seeds(catalogue, witness, cell_width_fraction=0.15):
    """Offset removed-spoke midpoint/center by2*fraction of the half-cell spoke.
    Produces author-parameter guesses ONLY. Does not freeze fields or claim
    coverage, convexity, quality, rail width, station spacing or shape acceptance.
    """
    if not 0 < cell_width_fraction < 0.5:
        raise ValueError('Invalid local width fraction')
    newids = set(witness['new_midpoints']) | set(witness['new_centers'])
    out = {i: np.asarray(catalogue['birth_parameters'][i], float).copy() for i in newids}
    center_ids = set(catalogue['centers'].values())
    for p in witness['patterns']:
        if 'transition_rotation' not in p:
            continue
        pair = p['transition_rotation']['removed']
        center = next((i for i in pair if i in center_ids))
        mid = next((i for i in pair if i != center))
        direction = out[center] - out[mid]
        out[mid] -= 2 * cell_width_fraction * direction
        out[center] += 2 * cell_width_fraction * direction
    return {i: value.tolist() for i, value in out.items()}

def forehead_pattern_prepare_birth_rows(existing_count, candidate_parameters, sample_birth, source_vertices, source_discrete):
    """Evaluate legitimate NEW births at final chosen author parameters.
    Reject existing IDs. Caller must separately enforce nonneutral-chain locus,
    surface connectivity/coverage and all geometry guards before materialization.
    """
    source = np.asarray(source_vertices)
    rows = {}
    positions = {}
    for i, parameter in candidate_parameters.items():
        if i < existing_count:
            raise ValueError('An existing frozen birth may not be reinitialized')
        sample = sample_birth(np.asarray(parameter, float))
        if sample is None:
            raise ValueError('Birth seed outside unambiguous source coverage')
        _, row = sample
        donors = np.asarray(row['donors'], int)
        weights = np.asarray(row['weights'], float)
        if len(donors) != len(weights) or len(set(donors)) != len(donors) or np.any(donors < 0) or np.any(donors >= len(source)) or (not np.isfinite(weights).all()) or np.any(weights < 0) or (abs(weights.sum() - 1) > 1e-08):
            raise ValueError('Invalid author birth row')
        for key, values in source_discrete.items():
            if key == '.select_vert':
                continue
            memberships = [values[d] for d, weight in zip(donors, weights) if weight > 0]
            if not memberships or any(value != memberships[0] for value in memberships[1:]):
                raise ValueError('Birth crosses discrete membership')
        rows[i] = copy.deepcopy(row)
        positions[i] = np.asarray(weights @ source[donors], np.float32).astype(float).tolist()
    return {'birth_rows': rows, 'positions': positions}

def forehead_pattern_exact_overlay_delta(before, after):
    before = np.asarray(before)
    after = np.asarray(after)
    if len(after) < len(before):
        raise ValueError('Original points must be retained')
    return np.flatnonzero(np.any(before != after[:len(before)], axis=1)).tolist()
