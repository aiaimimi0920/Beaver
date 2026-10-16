"""Exact expanded GLB accessor bytes shared by CPU regression manifests."""

from glb_affine import read_rows, require


def values(doc, binary, index):
    """Canonical raw component bytes plus semantics; ignore only storage layout.

    Normalized integers must retain both their bits and normalization flag.
    Float bytes also retain signed zero, with no rounding or tolerance comparison.
    Sparse implicit entries expand to zero bytes before exact comparison.
    """
    require(
        type(index) is int and 0 <= index < len(doc["accessors"]),
        "Invalid accessor",
    )
    accessor = doc["accessors"][index]
    kind, component = accessor["type"], accessor["componentType"]
    widths = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
    sizes = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
    require(
        kind in widths and component in sizes and not accessor.get("extensions"),
        "Unsupported accessor format",
    )
    normalized = accessor.get("normalized", False)
    require(
        isinstance(normalized, bool)
        and (not normalized or component in (5120, 5121, 5122, 5123)),
        "Invalid accessor normalization",
    )
    require(
        "bufferView" in accessor or accessor.get("byteOffset", 0) == 0,
        "Offset without buffer view",
    )
    count, size = accessor["count"], widths[kind] * sizes[component]
    require(type(count) is int and count > 0, "Invalid accessor count")

    def raw_rows(view, offset, length, packed=False):
        require(
            type(view) is int and 0 <= view < len(doc["bufferViews"]),
            "Invalid buffer view",
        )
        require(
            (doc["bufferViews"][view].get("byteOffset", 0) + offset) % sizes[component]
            == 0,
            "Misaligned component bytes",
        )
        return [
            bytes(r)
            for r in read_rows(
                doc, binary, view, offset, length, "<" + "B" * size, packed=packed
            )
        ]

    rows = (
        raw_rows(accessor["bufferView"], accessor.get("byteOffset", 0), count)
        if "bufferView" in accessor
        else [bytes(size)] * count
    )
    if "sparse" in accessor:
        sparse = accessor["sparse"]
        require(
            not sparse.get("extensions") and 0 < sparse["count"] <= count,
            "Invalid sparse accessor",
        )
        indices, replacements = sparse["indices"], sparse["values"]
        require(
            not indices.get("extensions") and not replacements.get("extensions"),
            "Unsupported sparse storage extensions",
        )
        require(
            indices["componentType"] in (5121, 5123, 5125), "Invalid sparse index type"
        )
        ids = [
            r[0]
            for r in read_rows(
                doc,
                binary,
                indices["bufferView"],
                indices.get("byteOffset", 0),
                sparse["count"],
                {5121: "<B", 5123: "<H", 5125: "<I"}[indices["componentType"]],
                packed=True,
            )
        ]
        require(ids == sorted(set(ids)) and ids[-1] < count, "Invalid sparse indices")
        for idx, row in zip(
            ids,
            raw_rows(
                replacements["bufferView"],
                replacements.get("byteOffset", 0),
                sparse["count"],
                packed=True,
            ),
        ):
            rows[idx] = row
    return kind, component, normalized, rows
