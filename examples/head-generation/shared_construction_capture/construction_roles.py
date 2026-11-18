"""Complete constructor-declared cross-row incidence, not semantic rail discovery."""


def incident_supports(faces, variable_point_ids):
    variables = list(variable_point_ids)
    wanted = set(variables)
    all_points = {pid for face in faces for pid in face["point_ids"]}
    if not wanted or len(wanted) != len(variables) or wanted - all_points:
        raise ValueError("Unknown/isolated/duplicate construction support variable")
    supports = [face for face in faces if wanted.intersection(face["point_ids"])]
    if any(len(face["point_ids"]) != 4 for face in supports):
        raise ValueError("Construction variable incidence contains a nonquad")
    return {"variable_point_ids": variables, "support_quads": supports,
            "coverage": "all actual variable-incident pretri construction faces",
            "field_and_solver_admission": "pending"}


def outer_correspondence(roles, faces, ordered_role_names):
    """The producer provides order and station correspondence; graph verifies all."""
    if len(ordered_role_names) < 3 or len(set(ordered_role_names)) != len(ordered_role_names):
        raise ValueError("Cross-band constructor group requires at least two declared bands")
    declared = {name for name, row in roles.items() if row["kind"] == "outer_constructor_row"}
    if set(ordered_role_names) != declared:
        raise ValueError("Incomplete declared outer-constructor comparison group")
    rows = []
    for name in ordered_role_names:
        row = roles[name]
        if not row["complete_declared_path"] or len(row["segments"]) != 1:
            raise ValueError("Constructor comparison row was deleted/split without replacement")
        rows.append(row["segments"][0])
    if len(set(map(len, rows))) != 1 or len({pid for row in rows for pid in row}) != sum(map(len, rows)):
        raise ValueError("Ambiguous constructor station correspondence")
    rungs = [[a, b] for lower, upper in zip(rows, rows[1:]) for a, b in zip(lower, upper)]
    expected = {frozenset(rung) for rung in rungs}
    edges = {frozenset((a, b)) for face in faces for a, b in zip(face["point_ids"], face["point_ids"][1:] + face["point_ids"][:1])}
    membership = {pid: row for row, points in enumerate(rows) for pid in points}
    observed = {edge for edge in edges if len(edge) == 2 and all(pid in membership for pid in edge)
                and len({membership[pid] for pid in edge}) == 2}
    if expected != observed or len(expected) != len(rungs):
        raise ValueError("Incomplete or extra actual cross-band constructor adjacency")
    variables = [pid for row in rows for pid in row]
    return {"family_id": "outer_constructor_all_neighbor_rows", "ordered_roles": list(ordered_role_names),
            "rungs": rungs, "coverage": "every actual cross-role edge in the entire declared constructor group",
            "scope": "outer constructor rows only; not five semantic facial rails or field admission",
            "support": incident_supports(faces, variables)}
