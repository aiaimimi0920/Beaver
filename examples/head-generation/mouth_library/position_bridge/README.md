# V15 authoritative-stream position bridge

This bounded tool transfers saved Blender mouth-position edits into the pinned
V15 GLB without invoking Blender's glTF exporter. It does not replace the face
generator, generate a new design, or certify shading/visual quality.

The code is public, but the model, editable `.blend`, and pinned baseline arrays
are deliberately not included. Supply the separately retained authorized V15
editable-entry/bridge data. Baseline pins bind this tool to one inspected asset
and Blender 4.3.2; changing a pin is not a supported way to admit another model.

## Run

Requires Python with NumPy and trusted Blender 4.3.2. Inspect the source first.
Keep automatic script execution disabled when opening the editable file.

```sh
python run_bridge.py --help
```

Pass the saved editable `.blend`, authoritative GLB, the immutable private
`baseline/` directory and a new output directory as documented by that command.
The runner invokes Blender with factory startup and automatic scripts disabled,
extracts data under strict checks, then runs the data-only patcher. The scripts
are not embedded in the model. Do not overwrite the original model or baseline.

## Preserved scope

- A no-op produces a GLB byte-identical to the original source.
- A permitted edit appends only changed POSITION streams. Every original BIN
  byte and original accessor remains intact; only the intended position
  references, new descriptors and buffer length change in JSON.
- Normals, morph normal deltas, tangents, UVs, colors, indices, embedded textures,
  materials, node information and non-mouth positions remain unchanged.
- Source float colors outside the usual range are preserved, not normalized.
- All eight absolute Blender position arrays (Basis plus seven keys) are read.
  Target deltas use the differences from the imported baseline, rather than
  reconstructing every delta from rounded absolute positions.
- Only the pinned five mouth rings and their existing split seam vertices may
  move. Seam vertices stay separate and must remain coincident. Retopology,
  added/deleted vertices, new shape keys and edits outside that set are rejected.
- Explicit normal/UV/color/material changes and raw archival-attribute changes
  are rejected. Derived native normals are not transferred. Transient selection
  attributes and ordering, and equivalent absent all-default sharp-edge/material
  indices, are handled narrowly; non-default changes remain rejected.

Nonzero position transfer uses float32 arithmetic and a checked endpoint error
bound; it is not a claim of exact native evaluated coordinates at every mix.
Preserving normals/tangents after changing geometry does **not** make those
shading vectors correct for the edit. Separate geometry, collision, normal,
rendering and artistic review is required before adopting a changed model.
Existing V15 color/normal import limitations are not repaired by this tool.

The extraction report is local consistency evidence, not a signed attestation.
Use the supplied runner on the actual saved file rather than hand-authoring a
report. No original Beaver/NPR project recovery, speech integration, deployment,
or complete glTF-spec conformance is claimed.

## Verification

Run asset-free parser, no-overwrite and position-boundary tests:

```sh
python -m unittest discover -s . -p 'test_*.py'
```

Private-input native verification separately exercised no-op, small synthetic
position edits, genuine Edit Mode Basis and MouthOpen edits, hostile/non-position
edits and multiple poses. Synthetic edited models are test fixtures, not an
adopted V16 design. The asset-free tests do not substitute for that evidence.
