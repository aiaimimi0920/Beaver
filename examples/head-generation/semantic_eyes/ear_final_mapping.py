"""Strictly map the retained procedural ear template onto a final source patch."""


def map_final_ear(polygons, expected, seeds, reverse=False):
    if len(polygons) != len(expected):
        raise ValueError('Original ear face count changed')
    mapping = dict(seeds)
    for actual, template in zip(polygons, expected):
        local = list(reversed(template)) if reverse else list(template)
        if len(actual) != len(local):
            raise ValueError('Original ear polygon layout changed')
        candidates = []
        for shift in range(len(actual)):
            cycle = tuple(actual[shift:]) + tuple(actual[:shift])
            candidate = dict(mapping)
            if any(i in candidate and candidate[i] != g for i,g in zip(local,cycle)):
                continue
            candidate.update(zip(local,cycle))
            if len(set(candidate.values())) == len(candidate):
                candidates.append(candidate)
        if len(candidates) != 1:
            raise ValueError('Ear source template mapping is ambiguous')
        mapping = candidates[0]
    if set(mapping) != set(range(40)) or len(set(mapping.values())) != 40:
        raise ValueError('Ear source template is incomplete')
    return [mapping[i] for i in range(40)]


def inset_vertex_weights(result):
    if 'authored_vertex_weights' in result:
        weights = result['authored_vertex_weights']
        if set(weights) != set(range(len(result['vertices']))):
            raise ValueError('Incomplete explicit source interpolation lineage')
        return weights
    weights = {i:{i:1.0} for i in range(40)}
    for patch in result['ogrid_details']:
        corners = patch['source_corners']
        t = patch['inset_fraction']
        parameters = ((t,t),(1-t,t),(1-t,1-t),(t,1-t))
        for vertex,(u,v) in zip(patch['new_vertex_ids'],parameters):
            weights[vertex] = dict(zip(corners,((1-u)*(1-v),u*(1-v),u*v,(1-u)*v)))
    if set(weights) != set(range(len(result['vertices']))):
        raise ValueError('Missing authored UV interpolation lineage')
    return weights
