"""Pure constructor oral-boundary identity composition, never spatial matching.

Callers must bind the supplied allocation tables to actual constructor objects.
These receipts confer no field-transfer, surgery, solver or model authority.
"""
from construction_topology import digest, stable_id
from construction_weld_contract import validate_weld


def _ids(values, label):
    if (type(values) is not list or not values
            or any(type(value) is not str or not value for value in values)
            or len(set(values)) != len(values)):
        raise ValueError("Missing/invalid/duplicate " + label)
    return values


def _seal(receipt):
    receipt["current_role_admission"] = False
    receipt["field_transfer_admission"] = False
    receipt["sha256"] = digest(receipt)
    return receipt


def expand_half_boundary(half_point_ids, seam_point_ids):
    """Mirror the authored open half ring, retaining each seam endpoint once."""
    half = _ids(half_point_ids, "half-boundary POINT IDs")
    if len(half) < 3 or seam_point_ids != [half[0], half[-1]]:
        raise ValueError("Explicit ordered half-boundary seam endpoints required")
    rows = [{"source_point_id": point, "side": 0 if i in (0, len(half) - 1) else 1}
            for i, point in enumerate(half)]
    rows.extend({"source_point_id": point, "side": -1} for point in half[-2:0:-1])
    return rows


def bind_oral_ring(half_point_ids, seam_point_ids, skin_namespace, oral_point_ids, ring0_indices):
    """Bind the real constructor's first allocated oral ring to shell Mirror IDs.

The bounded current producer allocates ring zero first, before interior rings.
No coordinates or geometric equality can establish the supplied identities.
"""
    expanded = expand_half_boundary(half_point_ids, seam_point_ids)
    oral = _ids(oral_point_ids, "oral constructor POINT IDs")
    if type(skin_namespace) is not str or not skin_namespace:
        raise ValueError("Explicit skin constructor namespace required")
    if (type(ring0_indices) is not list or any(type(i) is not int for i in ring0_indices)
            or ring0_indices != list(range(len(expanded))) or len(oral) < len(expanded)):
        raise ValueError("First oral ring allocation order/coverage changed")
    pairs = []
    for row, index in zip(expanded, ring0_indices):
        side = {0: "seam", 1: "original", -1: "reflected"}[row["side"]]
        skin = stable_id(skin_namespace, "mirrored_point", [row["source_point_id"], side])
        pairs.append({**row, "skin_point_id": skin, "oral_point_id": oral[index], "oral_index": index})
    skin_ids = _ids([row["skin_point_id"] for row in pairs], "mirrored skin boundary IDs")
    if set(skin_ids) & set(oral):
        raise ValueError("Skin/oral source identity domains overlap")
    return _seal({"schema": "oral_first_ring_identity_v1", "skin_namespace": skin_namespace,
        "half_point_ids": list(half_point_ids), "seam_point_ids": list(seam_point_ids),
        "oral_point_ids": list(oral), "ring0_indices": list(ring0_indices), "pairs": pairs,
        "scope": "declared constructor allocation relation; actual native binding required"})


def compose_donor_targets(boundary, join_receipt, weld_receipt):
    """Compose actual join/weld identities into exact post-Mesh donor indices.

Only declared skin/oral pair merges are admitted in this bounded slice. A
different weld elsewhere requires a separate receipt rather than a waiver.
"""
    expected_boundary = bind_oral_ring(boundary["half_point_ids"], boundary["seam_point_ids"],
        boundary["skin_namespace"], boundary["oral_point_ids"], boundary["ring0_indices"])
    if boundary != expected_boundary:
        raise ValueError("Changed oral allocation relation or receipt")
    source = join_receipt["source"]
    if digest(source) != join_receipt["source_sha256"]:
        raise ValueError("Changed native join source")
    source_ids = _ids(source["points"], "joined source POINT IDs")
    permutation = join_receipt["identity_maps"]["POINT"]
    if (type(permutation) is not list or any(type(i) is not int for i in permutation)
            or sorted(permutation) != list(range(1, len(source_ids) + 1))):
        raise ValueError("Incomplete native join POINT permutation")
    before = weld_receipt["before"]
    if (before["point_ids"] != [source_ids[i - 1] for i in permutation]
            or before["join_source_sha256"] != join_receipt["source_sha256"]):
        raise ValueError("Join-to-weld POINT binding changed")
    native = validate_weld(before, weld_receipt["native_target_pairs"], weld_receipt["after_weld"])
    if native != weld_receipt["quotient"]:
        raise ValueError("Changed native weld quotient receipt")
    post = validate_weld(before, weld_receipt["native_target_pairs"], weld_receipt["after_mesh"])
    if "mesh_quotient" in weld_receipt and post != weld_receipt["mesh_quotient"]:
        raise ValueError("Changed post-Mesh weld quotient")
    # Every actual oral allocation must have entered the joined source exactly once.
    if not set(boundary["oral_point_ids"]) <= set(source_ids):
        raise ValueError("Missing oral constructor allocation in joined source")
    wanted = {frozenset((row["skin_point_id"], row["oral_point_id"])): row for row in boundary["pairs"]}
    if len(wanted) != len(boundary["pairs"]):
        raise ValueError("Duplicate declared oral donor pair")
    targets = {}
    boundary_ids = {point for pair in wanted for point in pair}
    if not boundary_ids <= set(source_ids):
        raise ValueError("Missing declared skin/oral POINT identity")
    for index, indices in enumerate(post["point_sources"]):
        contributors = frozenset(before["point_ids"][i] for i in indices)
        if contributors in wanted:
            if contributors in targets or len(indices) != 2:
                raise ValueError("Duplicate/malformed oral weld donor class")
            targets[contributors] = index
        elif len(indices) != 1 or contributors & boundary_ids:
            raise ValueError("Unrelated/extra/missing skin/oral weld contributors")
    if set(targets) != set(wanted):
        raise ValueError("Incomplete declared skin/oral native weld coverage")
    rows = [{**row, "target_index": targets[frozenset((row["skin_point_id"], row["oral_point_id"]))]}
            for row in boundary["pairs"]]
    return _seal({"schema": "oral_donor_target_identity_v1", "boundary_sha256": boundary["sha256"],
        "join_source_sha256": join_receipt["source_sha256"], "native_quotient_sha256": native["sha256"],
        "post_mesh_quotient_sha256": post["sha256"], "rows": rows,
        "scope": "exact skin donor target identities only; fields and later surgery remain unadmitted"})
