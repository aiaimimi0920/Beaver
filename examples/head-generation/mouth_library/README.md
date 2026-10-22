# Researched mouth authoring slice

Four selectable designs are registered in `../face_components/catalog.json`:
`research-line`, `research-lip-band`, `research-round-muzzle`, and
`research-short-muzzle`. These are authored engineering presets, not four
scientifically established universal anatomical categories. Existing six mouth
options retain their prior path.

Use the six-component `plan.py` / `bundle.py` entry points. Optional
`--mouth-controls controls.json` accepts only the numeric fields owned by
`mouth_design.py`; it cannot change the preset kind or version. Plans do not
launch native tools or confer acceptance. Execute generated requests through
Beaver and retain the generation receipt and companion `.source-plan.json`.
Prepare the mandatory second request with `../source_layout/pack_request.py`
using `--source-plan`, `--generation-receipt`, `--workspace` and `--output`;
execute it through Beaver and retain its separate source-packaging receipt.
Then invoke `sdf_request.py` with both `--generation-receipt` and
`--source-packaging-receipt`. The final `verify_run` specification also needs
`source_packaging_receipt`; do not fabricate an aggregate receipt.
A fresh output prefix is required. Runtime GLB targets are ordered as:

1. MouthOpen
2. blink.L
3. blink.R
4. MouthRound
5. MouthSpread
6. MouthPucker

## Actual consumer

This slice consumes the generated GLB in the head-authoring showcase. Install
`runtime/mouth_review.gd` as
`res://showcase_extensions/researched_mouth_review.gd` using a guarded Beaver
file write. It extends the existing `head_review.gd`. Point the head-review scene
at this script for manual sliders and neutral reset. The four mouth state values
are bounded to [0, 1]; omitted fields reset to zero. They are not phoneme labels.
This does not implement the separate NPR performance-JSON or audio driver path.

## Native evidence

Install `runtime/mouth_probe.gd` as `researched_mouth_probe.gd` in the same
project folder, and install `runtime/mouth_capture_schedule.json` beside it.
The probe uses the same public manual-controller API for 92 recorded states:
front, both obliques, both profiles, render/white display, and selected blink
combinations. Its controls are intentionally locked during evidence capture;
use the manual scene for interactive review.

`verify_capture.py` checks exact schedule coverage, actual morph values, camera,
visibility, image hashes and dimensions, unchanged GLB identity, and installed
controller/probe/schedule identity. `face_components/verify_run.py` additionally
requires the existing 84-view eye and 18-view mouth regression manifests and
binds generation/SDF receipts to the requested composition.

The generator samples 81 open/round/spread/pucker combinations using actual
shape-key values. Export verification compares all triangle-referenced native
positions and six morph vectors with the GLB, permits export seam splits, binds
oriented triangle topology, and rechecks sampled triangles after export.
Loose vertices are excluded from this surface contract. None of these checks
proves global collision freedom, oral clearance, continuous-domain safety,
final artistic acceptance, or speech alignment.

See `DESIGN.zh-CN.md` for research and implementation boundaries.

## Controller behavior gate

Install `runtime/tests/test_researched_mouth_controller.gd` into the project `tests` directory through Beaver. It uses the actual controller and the verified combo892 GLB, not a mock. Its six tests cover combined weights, atomic rejection of invalid input, slider signals, omitted-value reset, the actual reset-button signal with blink preservation, and readiness guards. Update the explicit definition to another verified candidate when reusing this example.

`runtime/mouth_controller_review.tscn` and `runtime/validation-flow.json` provide the neutral UI flow for this historical candidate. Merge the flow into the existing validation configuration, preserving other flows and test directories; do not replace the project configuration with this fragment. Run Beaver normal code validation. The 92-state capture verifier does not replace GUT behavior tests. Signal tests do not by themselves prove physical mouse interaction; that is reviewed separately.


## Source-structure repair and acceptance boundary

The first five native mouth samples (886, 887, 891, 889 and 890) produced 970
reviewed frames. The native acceptance command then stopped at code-structure
validation, before executing the six new GUT tests. This was not a GUT pass.

With the user's subsequent repair approval, generated pipelines and parameter
modules were separated at semantic boundaries. See `../source_layout/` for
the exact reconstruction and two-native-job protocol. Refactoring historical
source changes its hashes; historical frame review does not prove current full
artifact-chain validity. Fresh source-bound samples and final native GUT/flow
results must be recorded before declaring this repair complete.


The post-refactor native samples are 893 (line), 894 (lip band), 895 (round
muzzle), 896 (short muzzle), plus cross-combination 892. Each has its own
successful generation, source-packaging and SDF receipts and 194 verified
source-bound captures (970 total). Physical slider, view-change and neutral-reset
interaction was retested on 892. The original pre-refactor observations remain in
`historical-pre-layout-evidence.json`; they are not substituted for the new chain.
Native GUT ran after the source and storage repairs: 12 tests passed (354 assertions),
including all six mouth-controller cases, with no skipped cases. See
`native-gut-evidence.json` for run identity and evidence hashes. The log still
reports nine deprecated `wait_frames` uses and five total orphan objects; this
is not a zero-warning claim. Final flow/product acceptance is separate.
