# Opt-in generated-source and actual six-key carrier

`POST_MORPH_SOURCE_CAPTURE_V1` is a **capture-only prerequisite**, not the shared
facial optimizer and not a successful model. Default generation does not call it.
It neither moves points nor enables another retile pass. The original independent
11-pose 1 mm source/author tests, all six fields, original/birth 0.5 mm budgets,
UV/category/source ownership, fixed diagonals, normal and final native/engine/
visual gates remain required. No acceptance gate is changed here.

## Actual consumer and ordering

`face_components.bundle.build_plan(..., shared_source_capture=
"POST_MORPH_SOURCE_CAPTURE_V1", source_seed=True)` passes through the existing
facial-surface and semantic-eye packagers. The initial supported recipe is the
explicit raw13 feature-flow family with researched-mouth and independent blink
builders. Unsupported policies/families or missing builders fail before execution.
The ordinary six-slot choices and their geometry remain unchanged.

1. After **existing** source shaping/retile and the editable-source save, but before
   runtime triangulation, freeze the generated source polygons, POINT/FACE/EDGE/
   CORNER attributes, three UV layers and corner editor state, deform memberships,
   material/edge/editor state, live original polygon-corner native normals and
   coordinate frame. Original normals stay separate from fixed-triangle normals.
   Bind source face owners from
   the live `authored_skin_ownership` constructor receipt, not numeric region
   assumptions or nearest geometry. Ordinary crease-group labels are not pins.
2. Add temporary native INT point/face/corner IDs. Actual runtime triangulation
   must preserve their full correspondence. Verify original source edges and
   complete oriented source-face triangle coverage, all original fields, and
   corner owner/side transport before the existing native normal setter.
3. Call the **same** `normals_split_custom_set` once, at its original stage.
   Bracket that call and record its exact generated sharp/custom-normal changes.
   `SourceAuthorSharp`, `SourceNPRNormal` and every other authored field must remain
   exact. Only native `custom_normal` (`CORNER/INT16_2D`) and `sharp_edge`
   (`EDGE/BOOLEAN`), plus the corresponding native edge-sharp flag, may change in
   this isolated phase. No blanket normal-layer exclusion is used. Native normal
   storage is documented by the [Blender manual](https://docs.blender.org/manual/en/latest/modeling/meshes/structure.html).
4. After the real `MouthOpen`, `blink.L`, `blink.R`, `MouthRound`, `MouthSpread`,
   `MouthPucker` builders, read the actual seven saved arrays including Basis.
   Require exact neutral/topology/field/normal continuity from the isolated normal
   phase. Keep absolute saved arrays and their derived deltas separately from the
   stored source `MouthOpenDelta`. No missing key is reconstructed or zero-filled.
5. Remove all temporary identities before runtime blend save/GLB export; retain
   the immutable capture in `generation_report.json.shared_source_capture`.
   One generated `try/finally` spans the entire interval, including triangulation
   and shape-builder failures, so native failures clean markers and abort rather
   than silently skipping capture.

Identity is the captured-source SHA-256 namespace plus a source-domain ordinal.
Runtime indices explicitly remap to these IDs; original face/corner identity and
actual fixed-triangle normals stay distinct. Several triangle corners can refer
to one original source corner without collapsing their native normals.

The modules are registered through an AST-read literal dictionary in the existing
retained `semantic_bundle.py`, never by executing that retained packager. This
reuses the existing source-seed/receipt chain and 64-output/1 MiB frame limits;
there are no new outputs, weakened limits, or separate generation service.
The hook carries one digest of the canonical resolved per-file source inventory;
the full inventory remains retained in the existing hash-bound source seed.
The tested `combo_reviewbootstrap` plan has only 355 bytes of serialized-frame
headroom in the current recipe. Longer otherwise-valid prefixes may be rejected
by the unchanged frame guard; this capture slice does not relax that limit or
claim arbitrary prefix lengths fit. The existing seed mechanism may allocate
additional text chunks for the retained sources; no native job is launched by
these packaging tests.

## Deliberate incomplete contracts

- The generated-source capture is **not** the earlier immutable RAW or AUTHOR
  baseline. These roles remain explicitly pending; no rebase is authorized.
- Source face/category ownership is real constructor evidence. Ordered full-rail,
  mirrored-seam, support-axis, cross-band comparison, and allowed-domain station
  roles are **not inferred** from it. They remain pending construction-time role
  capture. Derived birth identities likewise need their own explicit ledger.
- The stored actual triangulation is a capture, not proof that its diagonals are
  legal in all required poses. No solver/retile may consume this packet as admitted.
- Production packets contain actual live native normals, not final
  saved/reopened authority. Fresh generated-face marker propagation, save/reopen,
  GLB/engine identity and whole-model export still require production evidence.
- Repository unit/packaging tests use a typed native-API stand-in, **not Blender**.
  The separate real-native fixture verification below is not fresh-character or
  visual acceptance. Unsupported native types/channels must be reviewed, not dropped.

## Isolated real-native verification

A root-owned Blender 4.3.2 regression passed four cases: curved 3×4 and 5×7
procedural grids, each with all-default and nonuniform native editor fields.
It exercised real BMesh triangulation and stable-identity propagation, exact
source/UV/field retention, the original normal setter, six actual saved keys,
interrupted/completed marker cleanup, and exact blend save/reopen readback.
The immutable packet/source/normal-stage hashes and actual reopened key arrays
were checked independently. This establishes the native carrier mechanism only.
The full production face generator with this opt-in capture remains unrun;
RAW/AUTHOR admission, construction rail roles, shared optimization, whole-model
pose/GLB/engine checks and visual success are still pending.

## Focused checks

From the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=examples/head-generation/shared_source_capture \
  python3 -m unittest discover -s examples/head-generation/shared_source_capture/tests -v
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=examples/head-generation/face_components \
  python3 -m unittest test_feature_flow_recipe test_generated_source_size -v
npm run check:effective-lines
```

Coverage includes two different procedural curved grid sizes; actual six-array
capture, point/face storage permutations, source category/UV/deform/field/frame
failures, immutable source identity, missing keys, invalid triangle/corner mapping,
temporary-ID cleanup, and isolated native normal versus authored-sharp authority.
Two real resolved face configurations exercise the production packaging route,
source binding, exact lifecycle hooks and unchanged output/frame limits. These
packaging fixtures do not run either character generator in Blender.

## Native BMesh editor storage lifecycle

Before the isolated normal setter, capture may restore four native built-ins
that actual BMesh triangulation omits when every source value is False:
`.select_vert`, `.select_edge`, `.select_poly`, and `sharp_face`. This requires
exact known BOOLEAN/domain metadata, all-False immutable source values, actual
public-RNA default flags, and unchanged identity/topology/all other fields.
Real typed attributes are recreated, the entire snapshot is compared exactly,
and the original strict inventory/transport check runs again. The packet retains
the pre-normal restoration receipt separately from normal setter effects.
Actual BMesh also propagates POINT/EDGE selection and drops nondefault
`.select_poly`. The three selection channels are restored from exact authored
values only after typed source/native public-RNA agreement and complete oriented
point/face/corner/original-edge identity validation. New diagonal edges retain
their observed native selection; no original source ID is invented. Actual
before/after state and all three identity maps are recorded. Missing nondefault
POINT/EDGE storage, unbound generated selection storage, unknown/mistyped
omissions, nondefault sharp-face loss and unrelated changes still fail closed.
The entire actual result is compared exactly, then original strict transport
runs again. Source inventory and all authored values remain immutable; physical
storage and selection are restored, not waived or adopted as a new baseline.
