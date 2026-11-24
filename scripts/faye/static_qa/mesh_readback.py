"""Read-only evidence for static author polygons and native triangles."""
import argparse
from collections import Counter
import json
from math import acos, degrees, dist, hypot, isfinite, sqrt
from pathlib import Path


def edge(a, b):
    return tuple(sorted((a, b)))


def incidence(faces):
    return Counter(
        edge(a, b) for f in faces for a, b in zip(f, f[1:] + f[:1])
    )


def percentile(values, ratio):
    if not values:
        return None
    values = sorted(values)
    index = (len(values) - 1) * ratio
    start = int(index)
    part = index - start
    return values[start] * (1 - part) + values[min(start + 1, len(values) - 1)] * part


def triangle_quality(a, b, c):
    first = [y - x for x, y in zip(a, b)]
    second = [y - x for x, y in zip(a, c)]
    i, j, k = first
    l, m, n = second
    cross_length = hypot(j*n-k*m, k*l-i*n, i*m-j*l)
    distance_squared = dist(a, b)**2 + dist(b, c)**2 + dist(c, a)**2
    return 2 * sqrt(3) * cross_length / distance_squared if distance_squared else 0.0


def validate(document):
    """Enforce stable, explicitly recorded index space; do not infer welds."""
    vertices, polygons, triangles = (
        document[name] for name in ("vertices", "polygons", "triangles")
    )
    if not vertices or not polygons or not triangles:
        raise ValueError("vertices, polygons and native triangles must be nonempty")
    if any(len(p) != 3 or not all(isfinite(v) for v in p) for p in vertices):
        raise ValueError("each vertex must have three finite coordinates")
    for faces, triangle_only in ((polygons, False), (triangles, True)):
        for f in faces:
            if len(f) < 3 or (triangle_only and len(f) != 3) or len(set(f)) != len(f):
                raise ValueError("face has the wrong arity or a repeated vertex")
            if any(type(i) is not int or not 0 <= i < len(vertices) for i in f):
                raise ValueError("face index is not an integer in the vertex table")
    if len(triangles) != sum(len(p) - 2 for p in polygons):
        raise ValueError("native triangle count does not match the author polygons")
    return vertices, polygons, triangles


def summarize(vertices, polygons, triangles, axis=None):
    edges = incidence(polygons)
    directed = Counter(
        (a, b) for f in polygons for a, b in zip(f, f[1:] + f[:1])
    )
    lengths = {e: dist(vertices[e[0]], vertices[e[1]]) for e in edges}
    qualities = [triangle_quality(*(vertices[i] for i in t)) for t in triangles]
    output = {
        "author_faces": len(polygons),
        "native_triangles": len(triangles),
        "author_edges": len(edges),
        "total_author_edge_length": sum(lengths.values()),
        "author_edge_length_p10_p50_p90": [
            percentile(list(lengths.values()), p) for p in (0.1, 0.5, 0.9)
        ],
        "boundary_edges": sorted(e for e, n in edges.items() if n == 1),
        "nonmanifold_edges": sorted(e for e, n in edges.items() if n > 2),
        "inconsistent_interior_winding_edges": sorted(
            (a, b) for (a, b), n in edges.items()
            if n == 2 and directed[(a, b)] != directed[(b, a)]
        ),
        "minimum_native_triangle_quality": min(qualities),
        "zero_quality_native_triangles": sum(q <= 0 for q in qualities),
    }
    if axis is not None:
        norm = hypot(*axis)
        total = output["total_author_edge_length"]
        weighted_angles = []
        for (a, b), length in lengths.items():
            if length == 0:
                continue
            dot = sum((x - y) * k for x, y, k in zip(vertices[a], vertices[b], axis))
            ratio = min(1, abs(dot) / (norm * length))
            weighted_angles.append((degrees(acos(ratio)), length))
        output["axis_edge_length_share"] = {
            str(angle): sum(length for t, length in weighted_angles if t <= angle) / total
            if total else None for angle in (20, 35, 60)
        }
    return output


def compare(control, candidate, axis=None):
    if axis is not None and (
        len(axis) != 3 or not all(isfinite(x) for x in axis) or not hypot(*axis)
    ):
        raise ValueError("axis must contain three finite values and be nonzero")
    v0, p0, t0 = validate(control)
    v1, p1, t1 = validate(candidate)
    before = summarize(v0, p0, t0, axis)
    after = summarize(v1, p1, t1, axis)
    common = Counter(map(tuple, p0)) & Counter(map(tuple, p1))
    old_boundary = set(before["boundary_edges"])
    new_boundary = set(after["boundary_edges"])
    return {
        "control": before, "candidate": after,
        "unchanged_ordered_author_faces": sum(common.values()),
        "changed_shared_vertex_indices": sum(a != b for a, b in zip(v0, v1)),
        "vertex_table_sizes": [len(v0), len(v1)],
        "added_author_edges": sorted(incidence(p1).keys() - incidence(p0).keys()),
        "removed_author_edges": sorted(incidence(p0).keys() - incidence(p1).keys()),
        "boundary_indices_equal": new_boundary == old_boundary,
        "boundary_coordinates_equal": new_boundary == old_boundary and all(
            v0[i] == v1[i] for e in old_boundary for i in e
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("control", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--axis", nargs=3, type=float, metavar=("X", "Y", "Z"))
    args = parser.parse_args()
    try:
        datasets = [
            json.loads(p.read_text(encoding="utf-8"))
            for p in (args.control, args.candidate)
        ]
        result = compare(*datasets, axis=args.axis)
    except (OSError, KeyError, TypeError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
