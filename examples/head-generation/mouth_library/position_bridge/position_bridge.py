"""Data-only, fail-closed V15 position patcher. Never calls a glTF exporter."""

import argparse, copy, json, pathlib, struct
import numpy as np
from bridge_common import *
from bridge_glb import *


def validate_positions(positions, info, arrays, manifest):
    require(
        positions.dtype == np.float32
        and positions.shape == (8, len(arrays["shape_0"]), 3),
        "Expected eight float32 absolute position arrays",
    )
    require(np.isfinite(positions).all(), "Non-finite positions rejected")
    baseline = np.stack([arrays["shape_" + str(k)] for k in range(8)])
    allowed = np.zeros(positions.shape[1], dtype=bool)
    for r in manifest["rings"]:
        allowed[r["members"]] = True
    changed = np.any(positions != baseline, axis=2)
    require(
        not np.any(changed[:, ~allowed]), "Position edit outside allowed mouth rings"
    )
    # Every duplicate in these pinned rings coincides in all eight original key blocks.
    for r in manifest["rings"]:
        for station in r["stations"]:
            ids = station["coincident_source_vertices"]
            primary = station["primary_source_vertex"]
            require(
                np.array_equal(
                    baseline[:, ids],
                    np.repeat(baseline[:, primary : primary + 1], len(ids), axis=1),
                ),
                "Pinned baseline seam assumption failed",
            )
            require(
                np.array_equal(
                    positions[:, ids],
                    np.repeat(positions[:, primary : primary + 1], len(ids), axis=1),
                ),
                "Introduced split-seam divergence",
            )
    return baseline, changed, allowed


def check_extraction(extract_dir, info, arrays, manifest):
    p = pathlib.Path(extract_dir)
    report = json.loads((p / "extraction.json").read_text())
    require(
        report.get("format") == "faye-v15-position-extraction-v1",
        "Wrong extraction format",
    )
    require(
        report.get("source_sha256") == SOURCE_SHA, "Wrong extraction source identity"
    )
    require(report.get("validation") == "PASS", "Unvalidated extraction")
    require(
        report.get("shape_names") == [x["name"] for x in info["shapes"]],
        "Shape names changed",
    )
    require(
        report.get("baseline_pins") == BASELINE_PINS, "Extraction baseline pins differ"
    )
    require(
        file_sha(p / "positions.npz") == report["positions_npz_sha256"],
        "Extraction NPZ hash mismatch",
    )
    with np.load(p / "positions.npz", allow_pickle=False) as f:
        require(
            set(f.files) == {"positions", "indices", "source_vertex_indices"},
            "Unexpected extraction data",
        )
        pos = f["positions"]
        inds = f["indices"]
        ident = f["source_vertex_indices"]
    require(exact(inds, arrays["triangles"]), "Topology changed")
    require(
        exact(ident, arrays["attribute_V15_SOURCE_VERTEX_INDEX"]),
        "Source index identity changed",
    )
    require(report.get("vertex_groups") == info["groups"], "Vertex groups changed")
    return pos, report


def patch_bytes(source, positions, baseline_dir=None):
    require(sha(source) == SOURCE_SHA, "Wrong authoritative source SHA256")
    info, arrays, manifest = load_baseline(baseline_dir)
    baseline, changed, allowed = validate_positions(positions, info, arrays, manifest)
    g, b, text = parse_glb(source)
    require(
        len(g["meshes"]) == 1 and len(g["meshes"][0]["primitives"]) == 1,
        "Expected exactly one primitive",
    )
    p = g["meshes"][0]["primitives"][0]
    names = g["meshes"][0]["extras"]["targetNames"]
    require(
        p.get("mode", 4) == 4 and len(p["targets"]) == 7,
        "Primitive must have seven morph targets",
    )
    require(
        ["Basis"] + names == [x["name"] for x in info["shapes"]],
        "Source target names differ",
    )
    require(
        exact(
            accessor(g, b, p["indices"]).reshape(-1, 3).astype(np.int32),
            arrays["triangles"],
        ),
        "Source indices differ",
    )
    refs = [p["attributes"]["POSITION"]] + [x["POSITION"] for x in p["targets"]]
    streams = [accessor(g, b, i) for i in refs]
    for i, ref in enumerate(refs):
        a = g["accessors"][ref]
        require(
            a["componentType"] == 5126
            and a["type"] == "VEC3"
            and not a.get("normalized", False),
            "Position accessor must be nonnormalized float32 VEC3",
        )
        require(np.isfinite(streams[i]).all(), "Non-finite original stream")
        expected = to_blender(
            streams[0] if i == 0 else (streams[0] + streams[i]).astype(np.float32)
        )
        require(
            np.array_equal(expected, baseline[i]),
            "Pinned native import differs from authoritative stream",
        )
    dB = to_gltf(positions[0].astype(np.float64) - baseline[0].astype(np.float64))
    result_streams = [s.copy() for s in streams]
    masks = []
    mask = np.any(dB != 0, axis=1)
    masks.append(mask)
    result_streams[0][mask] = (streams[0][mask].astype(np.float64) + dB[mask]).astype(
        np.float32
    )
    for i in range(1, 8):
        dK = to_gltf(positions[i].astype(np.float64) - baseline[i].astype(np.float64))
        correction = dK - dB
        mask = np.any(correction != 0, axis=1)
        masks.append(mask)
        # Delta-on-original: do NOT reconstruct D from rounded imported K minus P.
        result_streams[i][mask] = (
            streams[i][mask].astype(np.float64) + correction[mask]
        ).astype(np.float32)
        require(
            exact(result_streams[i][~mask], streams[i][~mask]),
            "Unchanged delta rows not byte exact",
        )
    require(all(np.isfinite(s).all() for s in result_streams), "Float32 overflow")
    errors = []
    for i in range(8):
        absolute = (
            result_streams[0]
            if i == 0
            else (result_streams[0] + result_streams[i]).astype(np.float32)
        )
        err = float(
            np.max(
                np.abs(
                    to_blender(absolute).astype(np.float64)
                    - positions[i].astype(np.float64)
                )
            )
        )
        errors.append(err)
        require(
            err <= TARGET_TOLERANCE_M,
            "Float32 absolute target tolerance exceeded: " + str(err),
        )
    changed_streams = [i for i in range(8) if not exact(result_streams[i], streams[i])]
    report = {
        "format": "faye-v15-position-patch-report-v1",
        "validation": "PASS",
        "source_sha256": SOURCE_SHA,
        "synthetic_shape_quality_claim": False,
        "shading_validated": False,
        "source_bin_prefix_byte_exact": True,
        "unchanged_original_json_values_byte_exact": True,
        "all_non_position_stream_bytes_unchanged": True,
        "changed_absolute_vertices_by_key": dict(
            zip(["Basis"] + names, [int(x.sum()) for x in changed])
        ),
        "changed_position_streams": [],
        "target_absolute_max_error_m": dict(zip(["Basis"] + names, errors)),
        "target_absolute_tolerance_m": TARGET_TOLERANCE_M,
        "delta_formula": "Dnew = Dold + (Kedit - Kimport) - (Bedit - Bimport), float64 correction then float32 at changed rows only",
        "no_op_byte_identical": not changed_streams,
    }
    if not changed_streams:
        report.update(output_sha256=sha(source), output_bytes=len(source))
        return source, report
    new_accessors = []
    new_views = []
    binout = bytearray(b)
    patches = []
    spans = JsonSpans(text)
    for i in changed_streams:
        while len(binout) % 4:
            binout.append(0)
        raw = result_streams[i].astype("<f4", copy=False).tobytes()
        off = len(binout)
        binout.extend(raw)
        orig_a = g["accessors"][refs[i]]
        orig_v = g["bufferViews"][orig_a["bufferView"]]
        new_view = copy.deepcopy(orig_v)
        for key in ["byteStride", "extensions"]:
            new_view.pop(key, None)
        new_view.update(buffer=0, byteOffset=off, byteLength=len(raw))
        view_index = len(g["bufferViews"]) + len(new_views)
        new_views.append(new_view)
        new_a = copy.deepcopy(orig_a)
        new_a.update(
            bufferView=view_index,
            byteOffset=0,
            min=result_streams[i].min(axis=0).tolist(),
            max=result_streams[i].max(axis=0).tolist(),
        )
        accessor_index = len(g["accessors"]) + len(new_accessors)
        new_accessors.append(new_a)
        path = (
            ("meshes", 0, "primitives", 0, "attributes", "POSITION")
            if i == 0
            else ("meshes", 0, "primitives", 0, "targets", i - 1, "POSITION")
        )
        start, end = spans.spans[path]
        patches.append((start, end, str(accessor_index)))
        report["changed_position_streams"].append(
            {
                "name": (["Basis"] + names)[i],
                "old_accessor": refs[i],
                "new_accessor": accessor_index,
                "changed_rows": int(
                    np.any(
                        result_streams[i].view(np.uint32) != streams[i].view(np.uint32),
                        axis=1,
                    ).sum()
                ),
                "new_buffer_view": view_index,
            }
        )
    start, end = spans.spans[("buffers", 0, "byteLength")]
    patches.append((start, end, str(len(binout))))
    for key, items in [("accessors", new_accessors), ("bufferViews", new_views)]:
        _, end = spans.spans[(key,)]
        patches.append(
            (
                end - 1,
                end - 1,
                ","
                + ",".join(
                    json.dumps(x, separators=(",", ":"), allow_nan=False) for x in items
                ),
            )
        )
    output_text = text
    for start, end, replacement in sorted(patches, reverse=True):
        output_text = output_text[:start] + replacement + output_text[end:]
    jb = output_text.encode()
    jb += b" " * ((-len(jb)) % 4)
    bb = bytes(binout)
    bb += b"\x00" * ((-len(bb)) % 4)
    output = (
        struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(jb) + 8 + len(bb))
        + struct.pack("<II", len(jb), 0x4E4F534A)
        + jb
        + struct.pack("<II", len(bb), 0x004E4942)
        + bb
    )
    verify_output(source, output, changed_streams, result_streams)
    report.update(
        output_sha256=sha(output),
        output_bytes=len(output),
        original_bin_bytes=len(b),
        appended_bin_bytes=len(bb) - len(b),
        original_json_edit_ranges=[
            {"start": s, "end": e} for s, e, _ in sorted(patches)
        ],
    )
    return output, report


def verify_output(source, output, changed_streams, result_streams):
    old, ob, ot = parse_glb(source)
    new, nb, nt = parse_glb(output)
    require(nb[: len(ob)] == ob, "Original BIN prefix changed")
    normalized = copy.deepcopy(new)
    normalized["accessors"] = normalized["accessors"][: len(old["accessors"])]
    normalized["bufferViews"] = normalized["bufferViews"][: len(old["bufferViews"])]
    normalized["buffers"][0]["byteLength"] = old["buffers"][0]["byteLength"]
    p = normalized["meshes"][0]["primitives"][0]
    op = old["meshes"][0]["primitives"][0]
    p["attributes"]["POSITION"] = op["attributes"]["POSITION"]
    for i in range(7):
        p["targets"][i]["POSITION"] = op["targets"][i]["POSITION"]
    require(normalized == old, "Unexpected original JSON change")
    # Byte-exact untouched JSON subtrees, including node/material/resource metadata.
    os = JsonSpans(ot).spans
    ns = JsonSpans(nt).spans
    for path in os:
        allowed_change = (
            path == ("buffers", 0, "byteLength")
            or path[:3] == ("meshes", 0, "primitives")
            or path
            in [
                (),
                ("buffers",),
                ("buffers", 0),
                ("meshes",),
                ("meshes", 0),
                ("accessors",),
                ("bufferViews",),
            ]
        )
        if not allowed_change:
            a, z = os[path]
            c, d = ns[path]
            require(ot[a:z] == nt[c:d], "Untouched JSON bytes changed: " + str(path))
    np_ = new["meshes"][0]["primitives"][0]
    refs = [np_["attributes"]["POSITION"]] + [x["POSITION"] for x in np_["targets"]]
    for i, ref in enumerate(refs):
        require(
            exact(accessor(new, nb, ref), result_streams[i]), "Patched accessor differs"
        )
    for key, ref in op["attributes"].items():
        if key != "POSITION":
            require(
                exact(
                    accessor(old, ob, ref), accessor(new, nb, np_["attributes"][key])
                ),
                "Non-position attribute changed: " + key,
            )
    for i, target in enumerate(op["targets"]):
        for key, ref in target.items():
            if key != "POSITION":
                require(
                    exact(
                        accessor(old, ob, ref),
                        accessor(new, nb, np_["targets"][i][key]),
                    ),
                    "Non-position morph changed",
                )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=pathlib.Path)
    parser.add_argument(
        "--baseline-dir",
        type=pathlib.Path,
        help="Private pinned baseline data directory; defaults to adjacent baseline/",
    )
    parser.add_argument("--extraction", required=True, type=pathlib.Path)
    parser.add_argument(
        "--output-dir", required=True, type=pathlib.Path, help="Must not already exist"
    )
    args = parser.parse_args()
    require(
        not args.output_dir.exists(),
        "Output directory already exists; no overwrite permitted",
    )
    info, arrays, manifest = load_baseline(args.baseline_dir)
    positions, extract = check_extraction(args.extraction, info, arrays, manifest)
    source = args.source.read_bytes()
    output, report = patch_bytes(source, positions, args.baseline_dir)
    args.output_dir.mkdir(parents=False, exist_ok=False)
    report["extraction_json_sha256"] = file_sha(args.extraction / "extraction.json")
    exclusive_bytes(args.output_dir / "positions-only.glb", output)
    exclusive_json(args.output_dir / "patch-report.json", report)
    require(file_sha(args.source) == SOURCE_SHA, "Source changed during operation")
    print(
        json.dumps(
            {
                "result": "PASS",
                "no_op": report["no_op_byte_identical"],
                "output": str(args.output_dir / "positions-only.glb"),
                "sha256": report["output_sha256"],
            }
        )
    )


if __name__ == "__main__":
    main()
