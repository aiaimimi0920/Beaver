# Focused native mouth-controller validation

This harness executes `../runtime/mouth_review.gd` **unchanged** in a disposable
Godot project. It imports supplied local V15 and V11 GLBs using `GLTFDocument`
and reads/writes their actual `MeshInstance3D` blend shapes. No test replicates
the controller's implementation. Expected corrective samples are explicit:
`0 → 0`, `0.25 → 0.75`, `0.5 → 1`, `0.75 → 0.75`, `1 → 0`.

The inherited head-review host and `display.sync_morphs()` are small test-only
stand-ins. The latter records actual mesh values at each call to check update
order. This is **not** full Beaver/NPR application acceptance, visual review,
shader validation, speech/phoneme integration, or a replacement for those gates.

## Run

Requirements: Python 3.10+ standard library, Godot 4.6.3, and the two authorized
local model files. Models are neither bundled nor uploaded by this harness.

```sh
python examples/head-generation/mouth_library/controller_validation/run_controller_validation.py \
  --godot /usr/local/bin/godot \
  --v15 /local/path/faye-mid-corrective-v15.glb \
  --v11 /local/path/faye-lip-curve-v11.glb \
  --report /tmp/mouth-controller-validation.json
```

The command exits nonzero for assertion failures, Godot errors, missing reports
or nonzero Godot exits. It prints a JSON report with source/model SHA-256 hashes,
engine version, assertion count, failures, imported target names and real shape
samples. `--report` additionally saves that report and a sibling `.log` file.
Only temporary copies are changed, and the temporary project is removed afterward.

## Coverage

Each of three cases runs the same controller checks: actual V15, legacy V11
without the optional corrective, and V15 with its target order changed in a
temporary GLB so the corrective is index zero rather than six. Reordering updates
names, primitive target references, mesh weights and node weights together,
without changing any geometry buffers. Animated morph tracks are rejected by the
fixture builder rather than silently reordered incorrectly.

Checks cover:

- Real initialization and cached name-based target discovery.
- Exactly four public input channels, no added corrective slider or input key.
- The five opening values above, with simultaneous round/spread/pucker weights.
- Immediate public setter results and correction before display synchronization.
- Actual engine `_process` frames restoring deliberately changed mesh weights.
- Corrective inspection reading live mesh state rather than calculating a value.
- Actual slider signals, including all other mouth channels remaining intact.
- Invalid values and unknown/corrective keys rejected atomically, including mesh,
  stored weights, sliders, corrective and display synchronization count.
- Integer input, omitted-channel reset behavior, empty-state reset, and the real
  reset-button signal, all preserving both blink weights.
- Legacy absent-corrective output and unchanged four-channel behavior.
- An unready controller rejecting input without mutation.

`assertions.gd` owns observations, `controller_cases.gd` owns behavioral cases,
`validate_controller.gd` owns native imports and lifecycle, and the Python runner
owns disposable-project setup and reproducible evidence. `head_review_stub.gd`
is the deliberately limited host substitute.

## Verified result

Godot 4.6.3 completed all three native cases with **2,274 passing assertions**
and no Godot script/runtime errors. The V15 corrective was observed at index 6,
the legacy V11 correctly had no corrective, and the reordered V15 passed at
index 0. This result covers the isolated controller integration described above,
not full application or visual acceptance. Re-run with local authorized assets
to generate hashes and evidence for the current source revision.
