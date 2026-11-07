"""Read-only density comparison; one baseline-derived window frame for both meshes."""
import argparse
from collections import defaultdict
import json
from pathlib import Path

import numpy as np
from density_glb import read, require

WINDOWS = (
    ("chin", 0, .23, .25), ("mouth", .10, .27, .15),
    ("nose_band", .25, .43, .38), ("forehead", .47, 1, .38),
)


def welded_components(vertices, faces):
    _, weld = np.unique(np.round(vertices, 7), axis=0, return_inverse=True)
    parents = np.arange(int(weld.max()) + 1)

    def root(index):
        while parents[index] != index:
            parents[index] = parents[parents[index]]
            index = parents[index]
        return index

    for triangle in weld[faces]:
        parent = root(triangle[0])
        for index in triangle[1:]:
            parents[root(index)] = parent
    components = defaultdict(list)
    for index, triangle in enumerate(weld[faces]):
        components[root(triangle[0])].append(index)
    return [np.array(v, dtype=int) for v in components.values()], weld


def select_skin(data, components, areas):
    """Select by the retained facial UV chart and surface area, never face count."""
    uv = data["uv"][data["f"]]
    chart = ((uv[:, :, 0] >= .019999) & (uv[:, :, 0] < .56)).all(axis=1)
    candidates = [ids[chart[ids]] for ids in components]
    candidates = [ids for ids in candidates if len(ids)]
    require(bool(candidates), "No facial UV chart found")
    candidates.sort(key=lambda ids: float(areas[ids].sum()), reverse=True)
    largest = float(areas[candidates[0]].sum())
    total = sum(float(areas[ids].sum()) for ids in candidates)
    require(largest > 0 and largest > total / 2, "Ambiguous facial skin island")
    return candidates[0]


def stats(vertices, faces, indices):
    triangles = vertices[faces[indices]]
    if not len(triangles):
        return {"triangles": 0}
    edges = [triangles[:, (k + 1) % 3] - triangles[:, k] for k in range(3)]
    lengths = np.stack([np.linalg.norm(e, axis=1) for e in edges], axis=1)
    area = np.linalg.norm(np.cross(edges[0], -edges[2]), axis=1) / 2
    angles = []
    for k in range(3):
        a, b = edges[k], -edges[(k + 2) % 3]
        angles.append(np.degrees(np.arctan2(
            np.linalg.norm(np.cross(a, b), axis=1), np.sum(a * b, axis=1)
        )))
    minimum = np.min(angles, axis=0)
    return {
        "triangles": len(triangles), "surface_area_mm2": float(area.sum() * 1e6),
        "triangles_per_mm2": float(len(triangles) / max(area.sum() * 1e6, 1e-15)),
        "area_mm2_percentiles": np.percentile(area * 1e6, [5, 50, 95]).tolist(),
        "minimum_angle_deg": float(minimum.min()),
        "angle_under_5": int((minimum < 5).sum()),
        "edge_mm_percentiles": np.percentile(lengths * 1000, [5, 50, 95]).tolist(),
    }


def adjacent_ratios(faces, selected, weld, areas):
    uses = defaultdict(list)
    for index in selected:
        triangle = weld[faces[index]]
        for a, b in zip(triangle, np.roll(triangle, -1)):
            uses[tuple(sorted((int(a), int(b))))].append(int(index))
    require(all(len(ids) <= 2 for ids in uses.values()), "Nonmanifold front skin")
    ratios = [
        max(areas[a], areas[b]) / max(min(areas[a], areas[b]), 1e-15)
        for pair in uses.values() if len(pair) == 2 for a, b in [pair]
    ]
    require(bool(ratios), "No adjacent front skin triangles")
    return {
        "percentiles": np.percentile(ratios, [50, 90, 95, 99, 100]).tolist(),
        "over_4": int(sum(ratio > 4 for ratio in ratios)),
    }


def analyze(path, normalization=None):
    data = read(path)
    vertices, faces = data["v"], data["f"]
    triangles = vertices[faces]
    normals = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
    areas = np.linalg.norm(normals, axis=1) / 2
    centers = triangles.mean(axis=1)
    components, weld = welded_components(vertices, faces)
    skin = select_skin(data, components, areas)
    front = skin[normals[skin, 2] > 0]
    require(len(front) > 0, "No front-facing skin")
    if normalization is None:
        points = vertices[np.unique(faces[skin])]
        normalization = {
            "height_m": float(np.ptp(points[:, 1])),
            "origin_y_m": float(points[:, 1].min()),
            "source": "baseline referenced skin vertices, shared unchanged across comparison",
        }
    height, origin = normalization["height_m"], normalization["origin_y_m"]
    require(height > 0, "Collapsed face height")
    regions = {"front_skin": front}
    for name, lo, hi, half_width in WINDOWS:
        regions[name] = front[
            (centers[front, 1] >= origin + lo * height)
            & (centers[front, 1] < origin + hi * height)
            & (abs(centers[front, 0]) < half_width * height)
        ]
    return {
        "sha256": data["sha256"], "normalization": normalization,
        "total_triangles": len(faces), "skin_triangles": len(skin),
        "regions": {name: stats(vertices, faces, ids) for name, ids in regions.items()},
        "adjacent_area_ratios": adjacent_ratios(faces, front, weld, areas),
        "morph_names_only": data["morphs"],
    }


def compare(baseline, candidate):
    before = analyze(baseline)
    after = analyze(candidate, before["normalization"])
    return {
        "schema_version": 2, "baseline": before, "candidate": after,
        "windows": WINDOWS,
        "limits": [
            "Overlapping coordinate windows are coarse diagnostics, not semantic segmentation.",
            "Local density uses front-facing skin and actual 3D area, not screenshot line density.",
            "The facial UV chart/island convention is specific to the bounded shared generator.",
            "These v2 windows differ from earlier fixed-meter experiments; do not mix their counts.",
            "Triangle quality and morph names do not prove visual or deformation acceptance.",
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = compare(args.baseline, args.candidate)
    args.output.write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
