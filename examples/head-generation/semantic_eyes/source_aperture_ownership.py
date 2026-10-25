"""Topology-owned eye apertures in an authored half-face cage."""


def closed_interior_boundary_edges(faces, points, expected_components=1):
    """Identify closed boundary cycles wholly off the mirror seam.

    The supported authored half-face has one eye hole. The outer/mouth boundary
    touches the mirror seam and cannot own an eye crease, even where its
    coordinates happen to cross the eye's bounding rectangle.
    """
    uses = {}
    for face in faces:
        ids = tuple(face)
        if len(ids) < 3 or len(set(ids)) != len(ids):
            raise ValueError("Invalid source polygon boundary")
        for a, b in zip(ids, ids[1:] + ids[:1]):
            if not 0 <= a < len(points) or not 0 <= b < len(points):
                raise ValueError("Boundary vertex outside source cage")
            key = tuple(sorted((a, b)))
            uses[key] = uses.get(key, 0) + 1
    if any(count > 2 for count in uses.values()):
        raise ValueError("Nonmanifold source boundary")
    links = {}
    for (a, b), count in uses.items():
        if count == 1:
            links.setdefault(a, set()).add(b)
            links.setdefault(b, set()).add(a)
    remaining = set(links)
    accepted = []
    while remaining:
        component = {remaining.pop()}
        frontier = set(component)
        while frontier:
            frontier = {j for i in frontier for j in links[i]} - component
            component.update(frontier)
            remaining.difference_update(frontier)
        if not all(points[i][0] > 1e-7 for i in component):
            continue
        if len(component) < 3 or any(len(links[i]) != 2 for i in component):
            raise ValueError("Interior aperture is not a closed boundary cycle")
        accepted.append(component)
    if len(accepted) != expected_components:
        raise ValueError("Unexpected number of closed interior eye apertures")
    vertices = set().union(*accepted) if accepted else set()
    return {edge for edge, count in uses.items() if count == 1 and set(edge) <= vertices}
