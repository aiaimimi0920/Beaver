"""Conservative AVW diagonal trial, preserving all vertex fields and boundaries."""

import argparse
import json
from pathlib import Path

import numpy as np
from shapely.geometry import LineString, Polygon
from shapely.ops import unary_union

LOCKED_VERTICES = {7071, 7147, 7132, 7130, 7055}
POSES = [[0] * 7, [1, 0, 0, 0, 0, 0, 0], [1, 0, 0, 0, 1, 0, 0], [0, 0, 0, 1, 0, 1, 0]]


def orientation(uv, triangle):
    q = uv[triangle]
    return float(np.linalg.det(np.array([q[1] - q[0], q[2] - q[0]])))


def aspect(positions, triangles):
    q = positions[:, triangles]
    area = np.linalg.norm(
        np.cross(q[:, :, 1] - q[:, :, 0], q[:, :, 2] - q[:, :, 0]), axis=-1
    )
    edge = np.linalg.norm(q - np.roll(q, 1, axis=2), axis=-1)
    return float(np.max(edge.max(axis=-1) ** 2 / np.maximum(area, 1e-30)))


def edge_faces(faces):
    edges = {}
    for i, triangle in enumerate(faces):
        for a, b in zip(triangle, np.roll(triangle, -1)):
            edges.setdefault(tuple(sorted((int(a), int(b)))), []).append(i)
    return edges


def proposal(uv, fields, faces, edge, ids, edges, protected, locked, positions):
    a, b = edge
    if len(ids) != 2 or locked.intersection(edge):
        return None
    if LineString(uv[[a, b]]).intersection(protected).length > 1e-12:
        return None
    i, j = ids
    c = next(int(x) for x in faces[i] if x not in edge)
    d = next(int(x) for x in faces[j] if x not in edge)
    if c == d or tuple(sorted((c, d))) in edges:
        return None
    triangles = np.array([[c, d, a], [d, c, b]])
    if orientation(uv, triangles[0]) * orientation(uv, triangles[1]) <= 1e-24:
        return None
    old = unary_union([Polygon(uv[faces[i]]), Polygon(uv[faces[j]])])
    new = unary_union([Polygon(uv[t]) for t in triangles])
    if old.symmetric_difference(new).area > 1e-16:
        return None
    weights = np.linalg.solve(
        np.vstack([uv[[a, b, c]].T, np.ones(3)]), np.r_[uv[d], 1.0]
    )
    error = max(
        float(np.max(abs(weights @ v[[a, b, c]] - v[d]))) for v in fields.values()
    )
    if error > 1e-10:
        return None
    before, after = aspect(positions, faces[[i, j]]), aspect(positions, triangles)
    if after >= before * 0.95:
        return None
    for triangle in triangles:
        if orientation(uv, triangle) * orientation(uv, faces[i]) < 0:
            triangle[1], triangle[2] = triangle[2], triangle[1]
    record = dict(
        old_edge=[a, b], new_edge=[c, d], before=before, after=after, affine_error=error
    )
    return before - after, i, j, triangles, record


def refine(fields, uv, faces, protected, locked):
    faces = faces.copy()
    positions = np.array(
        [
            fields["POSITION"]
            + sum(
                (w * fields[f"morph{k}.POSITION"] for k, w in enumerate(ws)),
                np.zeros_like(fields["POSITION"]),
            )
            for ws in POSES
        ]
    )
    before = aspect(positions, faces)
    accepted = []
    for _ in range(20):
        edges = edge_faces(faces)
        candidates = [
            proposal(uv, fields, faces, e, ids, edges, protected, locked, positions)
            for e, ids in edges.items()
        ]
        candidates = [c for c in candidates if c is not None]
        if not candidates:
            break
        used = set()
        for _, i, j, triangles, record in sorted(candidates, key=lambda c: -c[0]):
            if i in used or j in used:
                continue
            faces[[i, j]] = triangles
            used.update((i, j))
            accepted.append(record)
    return faces, dict(
        accepted=accepted,
        region_faces=len(faces),
        before=before,
        after=aspect(positions, faces),
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--validation-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source, validation, output = (
        args.source.resolve(strict=True),
        args.validation_root.resolve(strict=True),
        args.output.resolve(),
    )
    if output.is_relative_to(source) or output.is_relative_to(validation):
        raise ValueError("Output must be a new directory outside source and validation")
    fields = dict(np.load(source / "full-fields.npz"))
    region = dict(np.load(source / "region.npz"))
    original = region["faces"]
    full = np.load(source / "full-geometry.npz")["faces"].copy()
    historical = np.load(
        validation / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz"
    )["faces"]
    protected = json.loads(
        (
            validation / "atd-band-protected-constraint-audit-20261112/CONSTRAINTS.json"
        ).read_text()
    )["shadow_protected_faces"]
    boundary = unary_union(
        [Polygon(region["uv"][historical[i]]).boundary for i in protected]
    )
    faces, report = refine(fields, region["uv"], original, boundary, LOCKED_VERTICES)
    lookup = {tuple(sorted(t)): i for i, t in enumerate(full)}
    for old, new in zip(original, faces):
        full[lookup[tuple(sorted(old))]] = new
    output.mkdir(parents=True, exist_ok=False)
    np.savez(output / "full-fields.npz", **fields)
    np.savez(output / "region.npz", faces=faces, uv=region["uv"])
    np.savez(output / "full-geometry.npz", faces=full)
    (output / "interface-splits.npz").write_bytes(
        (source / "interface-splits.npz").read_bytes()
    )
    (output / ".avw-isolated-run").write_text("Isolated affine diagonal trial\n")
    report.update(
        visual_accepted=False,
        native_generated=False,
        glb_verified=False,
        note="Four-pose proposal ranking only; run existing full validators and inspect renders",
    )
    (output / "EDGE-TRIAL.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report))


if __name__ == "__main__":
    main()
