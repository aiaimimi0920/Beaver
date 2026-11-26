"""Pure POINT/FACE/CORNER lineage proof for one explicit half-mesh Mirror.

IDs are opaque strings, typed by their enclosing domain and bound to one source
namespace. Native marker ordinals must already have been decoded through the
immutable source tables. No coordinates, nearest matches, implicit seam/pins,
modifier execution, or five-rail role discovery occurs here.
"""

from collections import Counter, defaultdict
import hashlib
import json

SIDES = ("original", "reflected")


def _digest(value):
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)
    return hashlib.sha256(raw.encode()).hexdigest()


def _rows(value, label):
    if type(value) is not list or not value:
        raise ValueError("Missing/non-list " + label)
    return value


def _ids(value, label):
    rows = _rows(value, label)
    if any(type(item) is not str or not item for item in rows) or len(rows) != len(set(rows)):
        raise ValueError("Invalid/duplicate " + label)
    return rows


def _index(value, label):
    if type(value) is not int or value < 0:
        raise ValueError("Invalid " + label + " storage index")
    return value


def _cycle(values):
    return min(tuple(values[i:] + values[:i]) for i in range(len(values)))


def _source_tables(source, seam_point_ids):
    points = set(_ids(source.get("point_ids"), "source POINT IDs"))
    faces, corners, directed, cycles = {}, {}, Counter(), set()
    for face in _rows(source.get("faces"), "source FACE rows"):
        if type(face) is not dict:
            raise ValueError("Invalid source FACE row")
        fid = face.get("face_id")
        if type(fid) is not str or not fid or fid in faces:
            raise ValueError("Invalid/duplicate source FACE ID")
        pids = _ids(face.get("point_ids"), "source face POINT cycle")
        cids = _ids(face.get("corner_ids"), "source face CORNER cycle")
        if len(pids) < 3 or len(pids) != len(cids) or set(pids) - points:
            raise ValueError("Degenerate/unbound source face cycle")
        key = min(_cycle(pids), _cycle(list(reversed(pids))))
        if key in cycles:
            raise ValueError("Duplicate source polygon regardless of winding")
        cycles.add(key)
        faces[fid] = list(zip(cids, pids))
        for cid, pid in zip(cids, pids):
            if cid in corners:
                raise ValueError("Duplicate original CORNER identity")
            corners[cid] = (fid, pid)
        directed.update(zip(pids, pids[1:] + pids[:1]))
    counts = Counter(tuple(sorted(edge)) for edge in directed.elements())
    if any(n > 1 for n in directed.values()) or any(n > 2 for n in counts.values()):
        raise ValueError("Inconsistent source winding/nonmanifold adjacency")
    if {pid for edge in counts for pid in edge} != points:
        raise ValueError("Isolated source POINT identity")
    if type(seam_point_ids) is not list or any(type(pid) is not str for pid in seam_point_ids):
        raise ValueError("Explicit constructor seam POINT list required")
    seam = set(seam_point_ids)
    if len(seam) != len(seam_point_ids) or seam - points:
        raise ValueError("Unknown/duplicate constructor seam POINT")
    boundary_points = {pid for edge, count in counts.items() if count == 1 for pid in edge}
    if seam - boundary_points:
        raise ValueError("Constructor seam POINT is not on the source boundary")
    return points, faces, corners, set(counts), seam


def _observed_vertices(observed, points):
    vertices, by_source = {}, defaultdict(list)
    rows = _rows(observed.get("vertices"), "observed POINT rows")
    for row in rows:
        if type(row) is not dict:
            raise ValueError("Invalid observed POINT row")
        index = _index(row.get("index"), "POINT")
        pid = row.get("source_point_id")
        if type(pid) is not str or pid not in points or index in vertices:
            raise ValueError("Unknown/duplicate observed POINT marker/index")
        vertices[index] = pid
        by_source[pid].append(index)
    if set(vertices) != set(range(len(rows))):
        raise ValueError("Incomplete contiguous observed POINT storage")
    return vertices, by_source


def _observed_faces(observed, vertices, source_faces, source_corners):
    face_map, corner_map = defaultdict(dict), defaultdict(dict)
    incidence, face_indices, corner_indices = defaultdict(set), set(), set()
    rows = _rows(observed.get("faces"), "observed FACE rows")
    for row in rows:
        if type(row) is not dict:
            raise ValueError("Invalid observed FACE row")
        index = _index(row.get("index"), "FACE")
        fid = row.get("source_face_id")
        if type(fid) is not str or fid not in source_faces or index in face_indices:
            raise ValueError("Unknown/duplicate observed FACE marker/index")
        vis = _rows(row.get("vertex_indices"), "observed face POINT indices")
        cis = _rows(row.get("corner_indices"), "observed face CORNER indices")
        cids = _ids(row.get("source_corner_ids"), "observed CORNER markers")
        if len(vis) != len(cids) or len(cis) != len(cids) or len(cids) != len(source_faces[fid]):
            raise ValueError("Changed observed face/corner cardinality")
        for vi, ci in zip(vis, cis):
            _index(vi, "POINT")
            _index(ci, "CORNER")
            if vi not in vertices or ci in corner_indices:
                raise ValueError("Missing POINT or duplicate observed CORNER index")
            corner_indices.add(ci)
        if len(set(vis)) != len(vis):
            raise ValueError("Degenerate observed face POINT cycle")
        pairs = [(cid, vertices[vi]) for cid, vi in zip(cids, vis)]
        if any(source_corners.get(cid) != (fid, pid) for cid, pid in pairs):
            raise ValueError("Mixed/corrupt original FACE/CORNER/POINT lineage")
        canonical = _cycle(pairs)
        matches = [side for side, sequence in zip(SIDES, (source_faces[fid], list(reversed(source_faces[fid]))))
                   if canonical == _cycle(sequence)]
        if len(matches) != 1:
            raise ValueError("Ambiguous/corrupt original cyclic corner winding")
        side = matches[0]
        if side in face_map[fid]:
            raise ValueError("Duplicate observed source FACE side occurrence")
        face_indices.add(index)
        face_map[fid][side] = index
        for cid, vi, ci in zip(cids, vis, cis):
            if side in corner_map[cid]:
                raise ValueError("Duplicate observed source CORNER side occurrence")
            corner_map[cid][side] = {"index": ci, "face_index": index, "point_index": vi}
            incidence[vi].add(side)
    if face_indices != set(range(len(rows))) or corner_indices != set(range(len(corner_indices))):
        raise ValueError("Incomplete contiguous observed FACE/CORNER storage")
    if set(face_map) != set(source_faces) or any(set(value) != set(SIDES) for value in face_map.values()):
        raise ValueError("Each original FACE requires exactly one original and one reflected occurrence")
    if set(corner_map) != set(source_corners) or any(set(value) != set(SIDES) for value in corner_map.values()):
        raise ValueError("Each original CORNER requires both side occurrences")
    return dict(face_map), dict(corner_map), incidence


def validate_mirror_lineage(source, observed, *, seam_point_ids, role_paths=None):
    """Validate decoded native marker rows without invoking Blender.

    Source: source_namespace, point_ids, faces[{face_id, point_ids, corner_ids}].
    Observed: same namespace, vertices[{index, source_point_id}], and
    faces[{index, source_face_id, vertex_indices, corner_indices,
    source_corner_ids}]. Indices cover their complete actual storage domains;
    input rows may be shuffled. role_paths is an optional explicit role->source
    POINT path mapping; validating it does not invent semantic handoffs.
    """
    if type(source) is not dict or type(observed) is not dict:
        raise ValueError("Typed source and observed envelopes required")
    namespace = source.get("source_namespace")
    if (type(namespace) is not str or len(namespace) != 64
            or any(c not in "0123456789abcdef" for c in namespace)
            or observed.get("source_namespace") != namespace):
        raise ValueError("Missing/mismatched source namespace SHA256")
    points, faces, corners, edges, seam = _source_tables(source, seam_point_ids)
    vertices, by_source = _observed_vertices(observed, points)
    face_map, corner_map, incidence = _observed_faces(observed, vertices, faces, corners)
    point_map, equivalence = {}, {}
    for pid in sorted(points):
        indices = by_source.get(pid, [])
        if pid in seam:
            if len(indices) != 1 or incidence[indices[0]] != set(SIDES):
                raise ValueError("Declared seam requires one POINT incident to both sides")
            point_map[pid] = dict.fromkeys(SIDES, indices[0])
            equivalence[pid] = {"observed_index": indices[0], "members": [
                {"source_point_id": pid, "side": side} for side in SIDES]}
        else:
            if len(indices) != 2 or {frozenset(incidence[i]) for i in indices} != {frozenset([s]) for s in SIDES}:
                raise ValueError("Off-seam merge/mixed lineage or isolated/missing POINT occurrence")
            point_map[pid] = {next(iter(incidence[i])): i for i in indices}
    roles = {} if role_paths is None else role_paths
    if type(roles) is not dict:
        raise ValueError("Explicit role path mapping required")
    for name, path in roles.items():
        _ids(path, "role POINT path")
        if type(name) is not str or not name or len(path) < 2 or set(path) - points:
            raise ValueError("Stale/unbound construction role station")
        if any(tuple(sorted((a, b))) not in edges for a, b in zip(path, path[1:])):
            raise ValueError("Construction role lost actual source adjacency")
    result = {"schema": "CONSTRUCTION_MIRROR_LINEAGE_V1", "source_namespace": namespace,
        "source_topology_sha256": _digest({"point_ids": sorted(points), "face_corner_point_cycles": faces}),
        "point_map": point_map, "face_map": face_map, "corner_map": corner_map,
        "seam_equivalence": equivalence, "validated_role_paths": {name: list(path) for name, path in roles.items()},
        "lineage_validated": True, "native_acceptance": "requires separately retained actual native receipt",
        "solver_binding": "pending semantic handoffs, source namespace mapping and independent field admission"}
    result["lineage_sha256"] = _digest(result)
    return result
