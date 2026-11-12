"""Geometry-derived smoothing only at the authored front/underside chin join."""


def mark_chin_normal_join(data, chain, excluded):
    """Tag interior chin attachment vertices; keep the lateral cap endpoint."""
    if len(chain) < 3 or len(set(chain)) != len(chain):
        raise ValueError('Invalid semantic chin chain')
    chosen = set(chain[:-1]) - set(excluded)
    if not chosen or any(i < 0 or i >= len(data.vertices) for i in chosen):
        raise ValueError('Invalid semantic chin attachment indices')
    attr = data.attributes.new('SmoothChinJoin', 'INT', 'POINT')
    for i in chosen:
        attr.data[i].value = 1


def shared_chin_normal_vertices(data, regions, excluded):
    """Never smooth a tagged point that also belongs to a different component."""
    attr = data.attributes.get('SmoothChinJoin')
    if attr is None or attr.domain != 'POINT' or attr.data_type != 'INT':
        raise ValueError('Missing semantic chin normal ownership')
    if len(attr.data) != len(data.vertices):
        raise ValueError('Chin ownership vertex count changed')
    selected = {i for i, item in enumerate(attr.data) if item.value == 1} - set(excluded)
    incident = {i: set() for i in selected}
    for poly in data.polygons:
        region = regions.data[poly.index].value
        for i in poly.vertices:
            if i in incident:
                incident[i].add(region)
    result = {i for i, owned in incident.items() if owned == {1, 2}}
    if not result:
        raise ValueError('Semantic chin join ownership did not survive generation')
    return result
