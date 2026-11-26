# Inactive source-field transport adapters

These modules are **not registered in generation or export**. They preserve
existing interfaces and do not adopt a new mesh or redefine acceptance.
Dependencies are Python/NumPy. The UV writer accepts a caller-owned Blender
mesh without importing or launching Blender.

- `source_field_accessor.py`: embedded GLB scalar/vector decoding, sparse
  substitution, signed/unsigned normalized integers. Matrices, compression,
  extension representations and external buffers fail closed. Normalization
  follows the Khronos glTF 2.0 accessor conversion table. Required extensions
  fail closed except `KHR_texture_transform`, which does not change accessor
  storage. Allowing that declaration does not validate texture/material behavior. Missing morph POSITION
  means zero delta, not missing source provenance. Each decoded accessor is
  capped at 64 MiB, accounting for normalized float64 output, before any array
  allocation. The missing morph-target zero fallback has the same byte bound.
- `source_field_serialization.py`: explicitly owned RGBA VEC4 becomes four
  homogeneous custom FLOAT SCALAR accessors. New JSON document; BIN unchanged.
  Caller retains its existing GLB packer and checks actual saved bytes. Standard
  COLOR_0, materials, UVs, morphs and normals are not rewritten.
- `source_field_identity.py`: actual source-normal/color buffers plus explicit
  staging-corner/native-loop/native-vertex maps; exact float32 words (including
  signed zero) and source hashes. Repeated IDs must carry identical data.
- `source_field_uv.py`: coordinate-only writer for explicitly false UV flags.
  No legacy lazy flag setters. Coordinates and every flag are read back.
  Existing true pins/selection fail before writing; they are never dropped.
  Names <=63 UTF-8 bytes and <=8 layers are explicit conservative adapter
  limits. Object-mode staging, attribute-name conflicts and allocated name/type
  are checked before coordinate writes; failed new allocations are rolled back.

Use existing `facial_quad_transport.py` for continuous original-quad weights.
Never average categorical integers, merge opposite UV/normal sides, rerun
native pose branching on new render vertices, or derive original deltas by
subtracting rounded saved targets. Preserve immutable source carriers.

The caller must independently hash actual original assets and authenticate the
export sidecar maps before calling recovery. These utilities check asserted
identity consistency and exact transport; a syntactically valid SHA256 string
is not proof that the asserted source asset was actually read.

## Separate gates

Authoritative fields can be exact float32 in saved mesh/custom GLB data and
owned resource metadata while preview/GPU normal buffers use finite precision.
Report each independently. Metadata is NOT shader binding. Source Color in
metadata does not imply renderer consumption. No shape/pose/UV threshold changes
are implied. Actual native/engine compatibility receipts remain private evidence,
not model assets or transferable acceptance for another mesh.

## Focused checks

With the project's NumPy environment, from repository root:

```sh
python -m unittest discover -s examples/head-generation/semantic_eyes \
  -p 'test_source_field_*.py' -v
npm run check:effective-lines
```

Synthetic tests cover malformed, sparse and normalized accessors, exact RGBA
words, semantic identity corruption, immutable document input and UV flags.
The UV mock verifies allocation policy, not Blender C++ behavior. Adoption still
requires actual native save/import/reopen and current engine resource roundtrip
on the caller's asset. No private model, vendor source or third-party assets
belong in these utilities.
