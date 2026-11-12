"""Identity-only constructor topology receipts; never infer geometry or anatomy."""
import hashlib
import json
from collections import Counter


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()).hexdigest()


def stable_id(namespace, domain, key):
    if not isinstance(key, (list, tuple)) or not key:
        raise ValueError("Explicit constructor allocation key required")
    return namespace + ":" + domain + ":" + digest(list(key))


def cycle(face):
    values = list(face)
    return min(tuple(values[i:] + values[:i]) for i in range(len(values)))


def topology(faces, point_count):
    directed, polygons = Counter(), set()
    for face in faces:
        if (len(face) < 3 or len(set(face)) != len(face)
                or any(type(i) is not int or not 0 <= i < point_count for i in face)):
            raise ValueError("Invalid constructor face ordinal")
        key = min(cycle(face), cycle(reversed(face)))
        if key in polygons:
            raise ValueError("Duplicate constructor polygon")
        polygons.add(key)
        directed.update(zip(face, list(face[1:]) + [face[0]]))
    if any(count != 1 for count in directed.values()):
        raise ValueError("Inconsistent oriented constructor incidence")
    return {tuple(sorted(edge)) for edge in directed}, {edge for edge in directed if edge[::-1] not in directed}


def role_segments(paths, live, edges):
    """Expose explicit breaks in declared construction paths, never bridge gaps."""
    result = {}
    for name, row in paths.items():
        path, pieces, current, breaks = row["slots"], [], [], []
        for slot in path:
            if slot not in live:
                if current:
                    pieces.append(current)
                    current = []
                breaks.append({"slot": slot, "reason": "constructor_allocation_not_in_final_faces"})
            elif current and tuple(sorted((current[-1], slot))) not in edges:
                pieces.append(current)
                breaks.append({"edge": [current[-1], slot], "reason": "declared_constructor_edge_absent"})
                current = [slot]
            else:
                current.append(slot)
        if current:
            pieces.append(current)
        if (row["closed"] and breaks and len(pieces) > 1
                and pieces[0][0] == path[0] and pieces[-1][-1] == path[-1]
                and tuple(sorted((path[-1], path[0]))) in edges):
            pieces = [pieces[-1] + pieces[0], *pieces[1:-1]]
        closed = bool(row["closed"] and not breaks)
        if closed and tuple(sorted((path[-1], path[0]))) not in edges:
            raise ValueError("Declared closed interface lost its closing edge")
        if any(len(piece) < 2 for piece in pieces):
            breaks.extend({"slot": piece[0], "reason": "isolated_declared_station"} for piece in pieces if len(piece) == 1)
        result[name] = {**row, "segments": [piece for piece in pieces if len(piece) >= 2],
                        "closed": closed, "breaks": breaks,
                        "complete_declared_path": not breaks}
    return result
