"""Add and audit actual final-mesh mouth targets; imports motion helpers above."""


def build_mouth_shapes(obj, ordinals, calibration):
    import itertools
    import numpy as np
    from mathutils import Vector

    mesh = obj.data
    original = [key.name for key in mesh.shape_keys.key_blocks]
    expected = ["Basis", "MouthOpen", "blink.L", "blink.R"]
    if original != expected:
        raise ValueError(
            "Mouth extension requires retained open and independent blink targets"
        )
    allowed = {1}
    for name in ("Oral cavity inner lips and back wall", "Tongue with thickness"):
        if name not in ordinals:
            raise ValueError("Missing mouth component: " + name)
        allowed.add(ordinals[name])
    members = set()
    for poly in mesh.polygons:
        if mesh.attributes["NormalRegion"].data[poly.index].value % 1000 in allowed:
            members.update(poly.vertices)
    base = np.array([tuple(v.co) for v in mesh.vertices], dtype=np.float64)
    deltas = {}
    for name in TARGETS:
        key = obj.shape_key_add(name=name)
        moves = np.zeros_like(base)
        for index in members:
            moves[index] = motion_delta(base[index], calibration, name)
            key.data[index].co += Vector(moves[index])
        key.value = 0.0
        deltas[name] = np.array([tuple(v.co) for v in key.data]) - base
    keys = mesh.shape_keys.key_blocks
    opened = np.array([tuple(v.co) for v in keys["MouthOpen"].data]) - base
    affected = np.any(
        np.stack([np.max(np.abs(d), axis=1) > 1e-12 for d in deltas.values()]), axis=0
    )
    # Blink regions must not receive any new mouth displacement. A tolerance here
    # only ignores numerical near-zero entries in the inherited blink target.
    overlap = 0
    for name in ("blink.L", "blink.R"):
        blink = np.array([tuple(v.co) for v in keys[name].data]) - base
        overlap += int(
            np.count_nonzero(affected & (np.linalg.norm(blink, axis=1) > 1e-7))
        )
    if overlap:
        raise ValueError("New mouth and blink supports unexpectedly overlap")
    mesh.calc_loop_triangles()
    tri = np.array([tuple(t.vertices) for t in mesh.loop_triangles], dtype=np.int32)
    all_triangles = tri.copy()
    tri = tri[np.any(affected[tri], axis=1)]

    def normals(points):
        a, b, c = points[tri[:, 0]], points[tri[:, 1]], points[tri[:, 2]]
        return np.cross(b - a, c - a)

    neutral = normals(base)
    min_area = float("inf")
    min_dot = float("inf")
    states = 0
    for weights in itertools.product((0.0, 0.5, 1.0), repeat=4):
        points = base + weights[0] * opened
        for weight, name in zip(weights[1:], TARGETS):
            points = points + weight * deltas[name]
        n = normals(points)
        areas = np.linalg.norm(n, axis=1) * 0.5
        dots = np.einsum("ij,ij->i", n, neutral)
        min_area = min(min_area, float(np.min(areas)))
        min_dot = min(min_dot, float(np.min(dots)))
        if np.any(areas <= 1e-12) or np.any(dots <= 0):
            raise ValueError("Mouth sampled pose folds or degenerates: " + str(weights))
        states += 1
    # Bind every native vertex and all six actual float32 shape-key deltas.
    # Export may split vertices at seams; the verifier permits duplicates only.
    actual = [
        np.array([tuple(v.co) for v in key.data]) - base for key in list(keys)[1:]
    ]
    used = sorted({int(i) for row in all_triangles for i in row})
    mapping = {old: new for new, old in enumerate(used)}
    binding = np.concatenate([base, *actual], axis=1)[used].tolist()
    topology = [[mapping[int(i)] for i in row] for row in all_triangles]
    return {
        "native_vertex_binding": binding,
        "native_triangles": topology,
        "binding_scope": "All triangle-referenced vertices; loose vertices excluded",
        "target_names": [key.name for key in keys][1:],
        "new_targets": list(TARGETS),
        "changed_vertices": {
            name: int(np.count_nonzero(np.linalg.norm(d, axis=1) > 1e-9))
            for name, d in deltas.items()
        },
        "max_displacement_m": {
            name: float(np.max(np.linalg.norm(d, axis=1))) for name, d in deltas.items()
        },
        "outside_owned_components_unchanged": True,
        "blink_overlap_vertices": overlap,
        "sampled_combinations": states,
        "sampled_triangles": len(tri),
        "minimum_sampled_area_m2": min_area,
        "minimum_sampled_normal_dot": min_dot,
        "scope": "81 sampled open/round/spread/pucker states on affected triangles; no continuous-domain, self-collision, oral-clearance or speech proof",
    }
