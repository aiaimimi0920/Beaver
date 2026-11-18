"""Exact native BMesh phase snapshots, including custom data and object incidence."""
from construction_markers import plain
from construction_topology import digest


def collections(bm):
    return {"POINT": list(bm.verts), "EDGE": list(bm.edges), "FACE": list(bm.faces),
            "CORNER": [loop for face in bm.faces for loop in face.loops]}


def layer_value(value):
    if hasattr(value, "uv"):
        return {name: plain(getattr(value, name)) for name in ("uv", "pin_uv", "select", "select_edge")}
    if hasattr(value, "radius") and hasattr(value, "use_root"):
        return {name: plain(getattr(value, name)) for name in ("radius", "use_root", "use_loose")}
    if hasattr(value, "items"):
        return sorted([[plain(k), plain(v)] for k, v in value.items()])
    return plain(value)


def custom_layers(access):
    result = []
    for kind in dir(access):
        if kind.startswith("_"):
            continue
        collection = getattr(access, kind)
        if not callable(getattr(collection, "keys", None)):
            continue
        for name in collection.keys():
            result.append((kind, name, collection[name]))
    return sorted(result, key=lambda row: row[:2])


def selection_rows(history):
    # Blender 4.3.2 BMEditSelIter has tp_iternext but no tp_iter. Indexed
    # BMEditSelSeq access preserves order without re-iterating that iterator.
    return [history[index] for index in range(len(history))]


def snapshot(bm, identity=None):
    """Return data and direct-object identity tables; never dereference retired objects."""
    rows = collections(bm)
    if identity is None:
        identity = {domain: {element: i for i, element in enumerate(values)} for domain, values in rows.items()}
    for domain, values in rows.items():
        if len(values) != len(identity[domain]) or any(element not in identity[domain] for element in values):
            raise ValueError("Native phase created/retired " + domain + " identities")
    point, edge, face, corner = (identity[key] for key in ("POINT", "EDGE", "FACE", "CORNER"))
    layers = {"POINT": custom_layers(bm.verts.layers), "EDGE": custom_layers(bm.edges.layers),
              "FACE": custom_layers(bm.faces.layers), "CORNER": custom_layers(bm.loops.layers)}
    def selected_identity(element):
        for domain, table in identity.items():
            if element in table:
                return [domain, table[element]]
        raise ValueError("Unbound BMesh editor selection identity")
    editor = {"selection_history": [selected_identity(e) for e in selection_rows(bm.select_history)],
              "selection_mode": sorted(bm.select_mode),
              "active_face": None if bm.faces.active is None else face[bm.faces.active]}
    result = {"editor": editor, "layers": {key: [[kind, name] for kind, name, _ in values] for key, values in layers.items()}, "domains": {}}
    for domain, values in rows.items():
        data = []
        for element in sorted(values, key=lambda item: identity[domain][item]):
            row = {"identity": identity[domain][element],
                   "custom": [[kind, name, layer_value(element[layer])] for kind, name, layer in layers[domain]]}
            if domain != "CORNER":
                row["flags"] = {name: getattr(element, name) for name in ("select", "hide", "tag")}
            if domain == "POINT":
                row.update(co=list(element.co), normal=list(element.normal))
            elif domain == "EDGE":
                row.update(points=[point[v] for v in element.verts], seam=element.seam, smooth=element.smooth)
            elif domain == "FACE":
                row.update(corners=[corner[loop] for loop in element.loops], normal=list(element.normal),
                           material_index=element.material_index, smooth=element.smooth)
            else:
                row["tag"] = getattr(element, "tag", None)
                row.update(point=point[element.vert], edge=edge[element.edge], face=face[element.face],
                           next=corner[element.link_loop_next], previous=corner[element.link_loop_prev])
            data.append(row)
        result["domains"][domain] = data
    # Serialization rejects nonfinite state; strings, booleans and signed zero stay typed.
    digest(result)
    return result, identity


def verify_noop(bm, state, identity):
    observed, _ = snapshot(bm, identity)
    if digest(observed) != digest(state):
        error = ValueError("Native dissolve changed geometry, incidence, fields or editor state without a lineage receipt")
        error.native_diagnostic = {"phase": "dissolve_noop", "before": state, "after": observed}
        raise error
    return {"scope": "complete BMesh element identity/connectivity and all Python-exposed native layers, flags and editor state no-op", "sha256": digest(observed),
            "unexposed_customdata_admitted": False}


def verify_dissolve(bm, state, identity, *, native_version):
    """Only the pinned operator's explicit per-corner scratch transition may differ."""
    from copy import deepcopy
    if tuple(native_version) != (4, 3, 2):
        raise ValueError("Dissolve scratch transition requires pinned Blender 4.3.2")
    observed, _ = snapshot(bm, identity)
    expected = deepcopy(state)
    transitions = []
    for row in expected["domains"]["CORNER"]:
        if type(row["tag"]) is not bool:
            raise ValueError("Native dissolve scratch tag is not an observed boolean")
        transitions.append({"corner_identity": row["identity"], "before": row["tag"], "after": True})
        row["tag"] = True
    if digest(observed) != digest(expected):
        error = ValueError("Native dissolve changed state beyond the explicit corner scratch-tag receipt")
        error.native_diagnostic = {"phase": "dissolve_scratch_transition", "before": state,
                                   "expected": expected, "after": observed}
        raise error
    return {"scope": "exact element identity/connectivity and exposed field no-op except the explicit native corner scratch tags",
            "native_version": list(native_version), "corner_scratch_transitions": transitions,
            "before": state, "after": observed, "before_sha256": digest(state), "after_sha256": digest(observed),
            "unexposed_customdata_admitted": False, "model_field_waiver": False}
