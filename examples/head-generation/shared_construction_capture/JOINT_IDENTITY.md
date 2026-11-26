# Dormant joint and directed-corner contracts

These utilities are not connected to generation, construction policy selection,
source inventories, or native runtime packaging. Existing default behavior is
unchanged. Importing them does not construct geometry or start Blender.

## Responsibilities

- `joint_binding.py` binds the producer's ordered ear points, explicit births,
  actual skin operands and side to a directed nine-cell footprint. Producer-local
  slots are a versioned source contract, not historical whole-model indices.
- `field_schema.py` requires exact stage-qualified field domains, names and types.
- `corner_ledger.py` identifies a corner by producer FACE identity and its directed
  POINT pair. Cyclic rotation preserves identity; winding reversal does not.
  Allocation records explicit source-corner weights and actual first-donor order.
- `corner_transport.py` checks indexed Mesh/BMesh layout and directed loop fields;
  `footprint_bridge.py` converts already-verified native identities to a footprint.
- `cap_fields.py` reads admitted BMesh fields. `cap_observer.py` records the actual
  two-triangle to quad replacement in the same live lease, including moved POINT
  identities and explicit corner births, without changing the geometry solver.

## Required caller boundary

These are low-level utilities, not a production admission API. A caller must
capture authentic POINT/FACE allocation IDs and ordered source donors while the
original elements are alive. A candidate must not invent or reseal the source
schema, boundary witnesses, or donor plan. No nearest-position or nearest-UV
matching, current enumeration fallback, or historical index substitution is valid.

Before observing, reject shape keys and unsupported schemas. Allocation currently
admits UV and FLOAT_COLOR corners only; BYTE_COLOR lacks an admitted quantization
contract and is rejected. The caller must reject preexisting construction marker
attributes, including the weld edge marker, before using `construction_markers.fields`,
which intentionally excludes those markers. Verify complete fields through the
entry/exit disposable-mesh probe, then exact saved/reopened readback. The probe
alone is not an admission check; its result must be compared by the caller.

The cap observer additionally refuses categorical/source seam ambiguities. These
are stricter identity admission conditions, not claims that the original geometry
solver imposed the same conditions. Original cap smooth assignment and new-face
selection/hide defaults are explicit field exceptions, not unchanged fields.

## Evidence and limits

CPU tests cover directed boundaries, side/order changes, source ID aliasing,
missing fields/faces, UV/categorical seams, donor-plan mutation and wrong births.
A separate synthetic, pre-key native experiment exercised the unchanged original
joint and return-cap functions, baseline/observer equality, directed corner
transport, and saved/reopened full fields. It did not test authentic complete
model generation, bilateral integration, shape keys, engine rendering or visual
acceptance. No user assets or private evidence files are required by these tests.

A fresh Blender 4.3.2 synthetic run verified the exact nine public module bytes
(the seven utilities plus the existing topology and field readers). Baseline and
observer fields matched; saved/reopened fields matched; all 44 directed corner
identities and 28 corner birth stencils were checked. The original cap operation
replaced two triangles with one quad without allocating cap points. Unsupported
BYTE_COLOR storage remained rejected. The run completed with cleanup confirmed.
Its evidence digest is
`a539ab3600e1de4152aa261c38a83d24915244453a65552f64bde815328a96a0`.
This is bounded synthetic byte-parity evidence, not an authentic constructor or
full-model acceptance claim. Future helper changes require renewed native parity;
CPU source extraction/reversal tests alone do not replace it.

Subsequent nasal replacement, regularization, compaction, spacing, chin and mouth
operations still need explicit composed POINT maps, FACE/CORNER birth/deletion
receipts and full-field transfer checks. These utilities do not skip those stages
or enable the currently unsupported later-role gate.
