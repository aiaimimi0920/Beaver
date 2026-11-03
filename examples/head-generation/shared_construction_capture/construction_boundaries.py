"""Constructor-issued aperture/seam facts plus exhaustive actual edge incidence."""
from collections import Counter


def boundary_facts(faces, point_ids, *, declared_seam=(), aperture_paths=None):
    points = set(point_ids)
    directed = Counter((a, b) for face in faces for a, b in zip(face, face[1:] + face[:1]))
    edges = Counter(tuple(sorted(edge)) for edge in directed.elements())
    if (len(points) != len(point_ids) or {p for edge in edges for p in edge} != points
            or any(count > 1 for count in directed.values()) or any(count > 2 for count in edges.values())):
        raise ValueError("Invalid constructor boundary incidence")
    seam = set(declared_seam)
    if seam - points:
        raise ValueError("Unbound declared constructor seam")
    aperture_edges = {}
    for name, row in (aperture_paths or {}).items():
        path = row["point_ids"]
        if len(path) < 2 or len(path) != len(set(path)) or set(path) - points:
            raise ValueError("Unbound constructor aperture path")
        pairs = list(zip(path, path[1:])) + ([(path[-1], path[0])] if row["closed"] else [])
        for a, b in pairs:
            edge = tuple(sorted((a, b)))
            if edges.get(edge) != 1 or edge in aperture_edges:
                raise ValueError("Declared aperture is not an unambiguous actual open boundary")
            aperture_edges[edge] = name
    records = []
    for edge, count in sorted(edges.items()):
        if count != 1:
            continue
        kind = ("declared_half_mirror_seam" if set(edge) <= seam else
                aperture_edges.get(edge, "unclassified_open_constructor_boundary"))
        records.append({"point_ids": list(edge), "kind": kind})
    excluded = sorted({pid for row in records if row["kind"] != "declared_half_mirror_seam" for pid in row["point_ids"]})
    return {"actual_boundary_edges": records, "open_boundary_except_mirror_seam": excluded,
            "authority": "actual face incidence and explicitly emitted aperture/seam construction paths",
            "not_authorized_by": ["radius", "whole_shell_tag", "NormalRegion"],
            "scope": "boundary membership only; not an immovable pin or complete anatomy/domain admission"}
