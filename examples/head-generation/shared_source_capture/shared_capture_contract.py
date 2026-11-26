"""Immutable source identity and exhaustive native-triangle correspondence.

This capture is not RAW/AUTHOR admission, a rail binder, or a mesh optimizer.
"""

from collections import Counter, defaultdict
import hashlib
import json

POLICY = "POST_MORPH_SOURCE_CAPTURE_V1"
KEYS = ("MouthOpen", "blink.L", "blink.R", "MouthRound", "MouthSpread", "MouthPucker")
MARKERS = {"POINT": "_BeaverCapturePoint", "FACE": "_BeaverCaptureFace", "CORNER": "_BeaverCaptureCorner"}
TOPOLOGY_ATTRIBUTES = {".edge_verts", ".corner_vert", ".corner_edge"}


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def same(a, b):
    """Finite native words stay exact, including signed zero and scalar type."""
    return encode(a) == encode(b)


def digest(value):
    return hashlib.sha256(encode(value).encode()).hexdigest()


def validate_vectors(rows, width, label):
    if any(len(row) != width or any(type(value) not in (int, float) for value in row) for row in rows):
        raise ValueError("Malformed " + label + " numeric vectors")
    encode(rows)


def validate_snapshot(mesh):
    counts = {"POINT": len(mesh["vertices"]), "EDGE": len(mesh["edges"]),
              "FACE": len(mesh["faces"]), "CORNER": len(mesh["corners"])}
    if not all(counts.values()):
        raise ValueError("Empty source mesh domain")
    for vertex in mesh["vertices"]:
        if len(vertex["co"]) != 3 or any(type(v) not in (int, float) for v in vertex["co"]):
            raise ValueError("Malformed native point")
    edge_pairs = []
    for edge in mesh["edges"]:
        pair = edge["vertices"]
        if (len(pair) != 2 or pair[0] == pair[1]
                or any(type(i) is not int or not 0 <= i < counts["POINT"] for i in pair)):
            raise ValueError("Malformed source edge")
        edge_pairs.append(frozenset(pair))
    if len(set(edge_pairs)) != len(edge_pairs):
        raise ValueError("Repeated source edge")
    if len(mesh["native_corner_normals"]) != counts["CORNER"]:
        raise ValueError("Native source corner normal coverage")
    validate_vectors(mesh["native_corner_normals"], 3, "native source normals")
    used = []
    for face in mesh["faces"]:
        vertices, loops = face["vertices"], face["loops"]
        if len(vertices) < 3 or len(set(vertices)) != len(vertices) or len(vertices) != len(loops):
            raise ValueError("Malformed source face/corner cycle")
        for vertex, loop in zip(vertices, loops):
            if (type(vertex) is not int or not 0 <= vertex < counts["POINT"]
                    or type(loop) is not int or not 0 <= loop < counts["CORNER"]
                    or mesh["corners"][loop]["vertex"] != vertex):
                raise ValueError("Invalid source face/corner identity")
        for i, loop in enumerate(loops):
            edge = mesh["corners"][loop]["edge"]
            if (type(edge) is not int or not 0 <= edge < counts["EDGE"]
                    or edge_pairs[edge] != frozenset((vertices[i], vertices[(i + 1) % len(vertices)]))):
                raise ValueError("Source corner edge incidence changed")
        used.extend(loops)
    if sorted(used) != list(range(counts["CORNER"])):
        raise ValueError("Missing/repeated source corner")
    for attr in mesh["attributes"].values():
        if attr["domain"] not in counts or len(attr["values"]) != counts[attr["domain"]]:
            raise ValueError("Native attribute domain coverage")
    topology = {".edge_verts": ("EDGE", "INT32_2D", [row["vertices"] for row in mesh["edges"]]),
                ".corner_vert": ("CORNER", "INT", [row["vertex"] for row in mesh["corners"]]),
                ".corner_edge": ("CORNER", "INT", [row["edge"] for row in mesh["corners"]])}
    for name, (domain, kind, values) in topology.items():
        attr = mesh["attributes"].get(name)
        if attr is not None and (attr["domain"] != domain or attr["type"] != kind or not same(attr["values"], values)):
            raise ValueError("Native topology attribute disagrees with mesh incidence")
    if len(mesh["uv_layers"]) != 3 or any(len(rows) != counts["CORNER"] for rows in mesh["uv_layers"].values()):
        raise ValueError("Three complete source UV layers required")
    encode(mesh)


def bind_source_owners(snapshot, ownership):
    """Use live constructor provenance, never region ordinals as role inference."""
    if (ownership.get("schema_version") != 1
            or ownership.get("role") != "authored_continuous_facial_shell"
            or ownership.get("validated") is not True or ownership.get("issues") != []
            or ownership.get("group_name") != "Continuous facial shell"
            or not ownership.get("object_name") or not ownership.get("skin_regions")):
        raise ValueError("Missing validated authored shell ownership")
    owners = {}
    members = [{"object_name": ownership["object_name"], "regions": ownership["skin_regions"]}]
    members += ownership["other_members"]
    for member in members:
        if not member["object_name"] or not member["regions"]:
            raise ValueError("Missing authored source member")
        for region in member["regions"]:
            if type(region) is not int or region in owners:
                raise ValueError("Ambiguous authored source region")
            owners[region] = member["object_name"]
    attr = snapshot["attributes"].get("NormalRegion")
    if not attr or (attr["domain"], attr["type"]) != ("FACE", "INT"):
        raise ValueError("Missing source category authority")
    if len(attr["values"]) != len(snapshot["faces"]):
        raise ValueError("Source category coverage")
    try:
        result = [owners[value] for value in attr["values"]]
    except KeyError as exc:
        raise ValueError("Unbound source category") from exc
    for face, owner in zip(snapshot["faces"], result):
        if owner != ownership["object_name"]:
            continue
        for index in face["vertices"]:
            if dict(snapshot["vertices"][index]["groups"]).get(ownership["group_name"]) != 1.0:
                raise ValueError("Source shell lost authored deform membership")
    return result


def freeze_source(snapshot, ownership, recipe):
    validate_snapshot(snapshot)
    source_hash = recipe.get("source_sha256")
    if (recipe.get("policy") != POLICY or type(source_hash) is not str or len(source_hash) != 64
            or any(character not in "0123456789abcdef" for character in source_hash)):
        raise ValueError("Missing resolved recipe SHA256 binding")
    payload = {"schema": POLICY, "mesh": snapshot, "ownership": ownership,
               "face_owners": bind_source_owners(snapshot, ownership), "recipe": recipe}
    raw = encode(payload)
    return {"source_json": raw, "source_sha256": hashlib.sha256(raw.encode()).hexdigest()}


def source_payload(capture):
    raw = capture["source_json"]
    if hashlib.sha256(raw.encode()).hexdigest() != capture["source_sha256"]:
        raise ValueError("Changed immutable source capture")
    return json.loads(raw)


def triangle_correspondence(source, runtime, identities):
    validate_snapshot(runtime)
    points, owners, corners = (identities[d] for d in ("POINT", "FACE", "CORNER"))
    if (len(points) != len(runtime["vertices"])
            or sorted(points) != list(range(len(source["vertices"])))
            or len(owners) != len(runtime["faces"])
            or len(corners) != len(runtime["corners"])):
        raise ValueError("Missing or duplicate source identity")
    if any(type(i) is not int for values in identities.values() for i in values):
        raise ValueError("Noninteger source identity")
    grouped, records = defaultdict(list), []
    for runtime_id, (face, owner) in enumerate(zip(runtime["faces"], owners)):
        if owner < 0 or owner >= len(source["faces"]) or len(face["vertices"]) != 3:
            raise ValueError("Invalid native triangle owner")
        original = source["faces"][owner]
        stable = [points[i] for i in face["vertices"]]
        source_loops = [corners[i] for i in face["loops"]]
        if len(set(stable)) != 3 or len(source_loops) != 3:
            raise ValueError("Degenerate native triangle identity")
        for vertex, loop in zip(stable, source_loops):
            if (loop not in original["loops"]
                    or source["corners"][loop]["vertex"] != vertex):
                raise ValueError("Native corner lost source owner/side")
        grouped[owner].append(stable)
        records.append({"runtime_face": runtime_id, "source_face": owner,
                        "source_vertices": stable, "source_corners": source_loops})
    for owner, face in enumerate(source["faces"]):
        triangles = grouped[owner]
        if len(triangles) != len(face["vertices"]) - 2:
            raise ValueError("Incomplete source face triangulation")
        counts = Counter((a, b) for triangle in triangles
                         for a, b in zip(triangle, triangle[1:] + triangle[:1]))
        boundary = Counter(zip(face["vertices"], face["vertices"][1:] + face["vertices"][:1]))
        if any(count > 1 for count in counts.values()):
            raise ValueError("Repeated native triangle edge")
        exposed = Counter({edge: count for edge, count in counts.items()
                           if (edge[1], edge[0]) not in counts})
        if exposed != boundary:
            raise ValueError("Native triangle winding/coverage changed")
    return records


def validate_transport(source, runtime, identities):
    """Verify unchanged source fields through triangulation, including old edges."""
    records = triangle_correspondence(source, runtime, identities)
    maps = dict(identities)
    source_edges = {tuple(sorted(row["vertices"])): i for i, row in enumerate(source["edges"])}
    edge_map = [source_edges.get(tuple(sorted(maps["POINT"][i] for i in e["vertices"])))
                for e in runtime["edges"]]
    if sorted(i for i in edge_map if i is not None) != list(range(len(source["edges"]))):
        raise ValueError("Original source edges lost or duplicated")
    maps["EDGE"] = edge_map
    for domain, key in (("POINT", "vertices"), ("FACE", "faces"), ("EDGE", "edges")):
        for row, original in zip(runtime[key], maps[domain]):
            if original is None:
                continue
            a = {k: v for k, v in row.items() if k not in ("vertices", "loops")}
            b = {k: v for k, v in source[key][original].items() if k not in ("vertices", "loops")}
            if not same(a, b):
                raise ValueError("Changed source " + domain + " values")
    if set(source["attributes"]) != set(runtime["attributes"]):
        raise ValueError("Changed native attribute inventory")
    for name, before in source["attributes"].items():
        after = runtime["attributes"][name]
        if (before["domain"], before["type"]) != (after["domain"], after["type"]):
            raise ValueError("Changed attribute domain/type")
        if name in TOPOLOGY_ATTRIBUTES:
            continue
        for value, original in zip(after["values"], maps[before["domain"]]):
            if original is not None and not same(value, before["values"][original]):
                raise ValueError("Changed source attribute: " + name)
    if set(source["uv_layers"]) != set(runtime["uv_layers"]):
        raise ValueError("Changed source UV inventory")
    for name, rows in source["uv_layers"].items():
        if not same(runtime["uv_layers"][name], [rows[i] for i in maps["CORNER"]]):
            raise ValueError("Changed source UV side/pin/editor state")
    if (source["materials"] != runtime["materials"]
            or not same(source["object_matrix_world"], runtime["object_matrix_world"])):
        raise ValueError("Changed source materials/frame")
    return records, edge_map


def complete_capture(capture, runtime, identities, shapes, normals):
    original = source_payload(capture)
    normal_stage = None
    if "normal_stage_json" in capture:
        raw = capture["normal_stage_json"]
        if hashlib.sha256(raw.encode()).hexdigest() != capture["normal_stage_sha256"]:
            raise ValueError("Changed native normal stage capture")
        stage = json.loads(raw)
        if (stage["source_sha256"] != capture["source_sha256"] or stage["identities"] != identities
                or not same(stage["after"], runtime) or not same(stage["actual_corner_normals"], normals)):
            raise ValueError("Mesh/normal fields changed after native normal setter")
        records, edges = validate_transport(original["mesh"], stage["before"], identities)
        normal_stage = {"sha256": capture["normal_stage_sha256"], "changes": stage["changes"],
                        "before_sha256": digest(stage["before"]), "after_sha256": digest(runtime),
                        "requested_corner_normals": stage["requested_corner_normals"],
                        "scope": "isolated existing native setter; save/reopen not verified"}
        normal_stage["transport_attribute_restoration"] = stage.get("transport_attribute_restoration")
    else:
        records, edges = validate_transport(original["mesh"], runtime, identities)
    if list(shapes) != ["Basis", *KEYS]:
        raise ValueError("Actual six saved shape keys required in declared order")
    count = len(runtime["vertices"])
    if any(len(value) != 3 or any(type(v) not in (int, float) for v in value)
           for rows in shapes.values() for value in rows):
        raise ValueError("Malformed actual saved key vector")
    if any(len(rows) != count for rows in shapes.values()):
        raise ValueError("Saved shape key point coverage")
    if not same(shapes["Basis"], [row["co"] for row in runtime["vertices"]]):
        raise ValueError("Saved Basis differs from captured neutral")
    if len(normals) != len(runtime["corners"]):
        raise ValueError("Native corner normal coverage")
    validate_vectors(normals, 3, "native fixed-triangle normals")
    if not same(normals, runtime["native_corner_normals"]):
        raise ValueError("Actual normal argument differs from native snapshot")
    inverse = {source: current for current, source in enumerate(identities["POINT"])}
    saved = {name: [rows[inverse[i]] for i in range(count)] for name, rows in shapes.items()}
    delta = {name: [[a - b for a, b in zip(value, neutral)]
                    for value, neutral in zip(saved[name], saved["Basis"])] for name in KEYS}
    packet = {"schema": POLICY, "source": original, "source_sha256": capture["source_sha256"],
              "runtime": runtime, "runtime_to_source": identities, "runtime_edge_to_source": edges,
              "fixed_triangles": records, "saved_keys_by_source_point": saved,
              "saved_minus_basis_by_source_point": delta, "actual_native_corner_normals": normals,
              "normal_authority": "live post-morph native mesh; save/reopen pending",
              "native_normal_generation": normal_stage,
              "identity_namespace": capture["source_sha256"],
              "identity_rule": "namespace + domain + source ordinal; runtime ordinals are remapped",
              "native_ready": False, "full_model_success": False,
              "raw_author_binding": "pending independent reference roles",
              "rail_role_binding": "pending construction-time ordered station capture",
              "scope": "source carrier only; no redistribution, retile, donor or gate admission"}
    encode(packet)
    return packet
