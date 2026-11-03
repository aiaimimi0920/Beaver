"""Identity-only proof for native welds without face splitting or new geometry."""
from collections import defaultdict
from construction_topology import cycle, digest


def exact_index(value, size, label):
    if type(value) is not int or not 0 <= value < size:
        raise ValueError("Invalid " + label + " ordinal")
    return value


def terminal_map(point_count, native_pairs):
    """Resolve the observed native map, rejecting chains/cycles/aliases silently lost."""
    mapping = {}
    for pair in native_pairs:
        if len(pair) != 2:
            raise ValueError("Malformed native weld target pair")
        source, target = (exact_index(v, point_count, "target map") for v in pair)
        if source == target or source in mapping:
            raise ValueError("Self/duplicate native weld source")
        mapping[source] = target
    if any(target in mapping for target in mapping.values()):
        raise ValueError("Chained native weld target map is not admitted")
    result = {}
    for point in range(point_count):
        seen, target = set(), point
        while target in mapping:
            if target in seen:
                raise ValueError("Cyclic native weld target map")
            seen.add(target)
            target = mapping[target]
        result[point] = target
    return result


def projected_face(points, corners, targets):
    """Match native createface: retain the old loop whose outgoing edge survives."""
    if len(points) != len(corners) or len(points) < 3 or len(set(points)) != len(points):
        raise ValueError("Invalid original weld polygon")
    for i, source in enumerate(points):
        target = targets[source]
        if target != source and target in points:
            j = points.index(target)
            if (j - i) % len(points) not in (1, len(points) - 1):
                raise ValueError("Weld requires unreceipted native face split before collapse")
    mapped = [targets[p] for p in points]
    kept = [(corners[i], point) for i, point in enumerate(mapped) if point != mapped[(i + 1) % len(mapped)]]
    if len({p for _, p in kept}) != len(kept):
        raise ValueError("Weld requires unreceipted nonadjacent face split")
    return kept


def validate_weld(before, native_pairs, after):
    """Prove complete native targetmap quotient and explicit loop-copy provenance.

    Duplicate collapsed faces have a complete candidate set and observed donor;
    this is topology lineage, not permission to combine conflicting source fields.
    """
    count = len(before["point_ids"])
    if count == 0 or len(set(before["point_ids"])) != count:
        raise ValueError("Missing/duplicate source weld points")
    targets = terminal_map(count, native_pairs)
    observed = after["point_origins"]
    for origin in observed:
        exact_index(origin, count, "post-weld point")
    if len(observed) != len(set(observed)) or set(observed) != set(targets.values()):
        raise ValueError("Native weld survivors differ from observed target map")
    groups, projections, retired = defaultdict(list), {}, []
    all_corners = []
    for index, face in enumerate(before["faces"]):
        for point in face["points"]:
            exact_index(point, count, "source face point")
        all_corners.extend(face["corners"])
        kept = projected_face(face["points"], face["corners"], targets)
        projections[index] = kept
        if len(kept) < 3:
            retired.append(index)
        else:
            groups[cycle([p for _, p in kept])].append(index)
    if any(type(v) is not int for v in all_corners) or sorted(all_corners) != list(range(len(all_corners))):
        raise ValueError("Duplicate source weld corners")
    actual_groups, used_donors, face_records = set(), set(), []
    for face in after["faces"]:
        donor = exact_index(face["face_origin"], len(before["faces"]), "post-weld face")
        if donor in used_donors:
            raise ValueError("Native weld duplicated a face donor")
        used_donors.add(donor)
        points = [observed[exact_index(v, len(observed), "post face point")] for v in face["points"]]
        corners = face["corner_origins"]
        if len(points) != len(corners) or any(type(v) is not int for v in corners):
            raise ValueError("Malformed post-weld corner identities")
        key = cycle(points)
        if key not in groups or key in actual_groups or donor not in groups[key]:
            raise ValueError("Unexplained native weld face birth/loss/orientation")
        actual_groups.add(key)
        if cycle(list(zip(corners, points))) != cycle(projections[donor]):
            raise ValueError("Native weld outgoing corner donor differs from mapped source ring")
        face_records.append({"source_face": donor, "source_face_candidates": groups[key],
                             "source_corners": corners, "source_target_points": points})
    if actual_groups != set(groups):
        raise ValueError("Native weld omitted a nondegenerate source face class")
    edge_groups = defaultdict(list)
    for index, edge in enumerate(before["edges"]):
        if len(edge) != 2:
            raise ValueError("Malformed original weld edge")
        a, b = (targets[exact_index(v, count, "source edge")] for v in edge)
        if a != b:
            edge_groups[tuple(sorted((a, b)))].append(index)
    actual_edges = []
    for edge in after["edges"]:
        if len(edge) != 2:
            raise ValueError("Malformed native weld edge")
        pair = tuple(sorted(observed[exact_index(v, len(observed), "post edge")] for v in edge))
        if pair[0] == pair[1]:
            raise ValueError("Native weld retained collapsed edge")
        actual_edges.append(pair)
    if len(actual_edges) != len(set(actual_edges)) or set(actual_edges) != set(edge_groups):
        raise ValueError("Native weld edge quotient differs from source target map")
    point_sources = defaultdict(list)
    for point, target in targets.items():
        point_sources[target].append(point)
    receipt = {"source_sha256": digest(before), "native_target_pairs": native_pairs,
               "point_sources": [point_sources[p] for p in observed], "point_target_origins": observed,
               "faces": face_records, "edges": [edge_groups[e] for e in actual_edges],
               "retired_degenerate_faces": retired,
               "retired_duplicate_faces": sorted(set(range(len(before["faces"]))) - set(retired) - used_donors),
               "scope": "native point/edge/face/corner quotient only; field donor admission remains pending"}
    receipt["sha256"] = digest(receipt)
    return receipt
