# F8: Frozen PNG delivery to the object-task model

Date: 2026-09-26.

## Usable outcome

The existing candidate PNG rework workflow now delivers the frozen image as an
actual `inputImage` item when the executing model calls
`beaver_object_attempt` with `operation: inputFile`. Previously the tool reply
placed image base64 inside a text JSON result. The task already retained the PNG
hash, numbered normalized regions and owner feedback in its frozen input and
prompt; the missing model-facing delivery boundary is now connected.

The production candidate form, Desktop command, durable recovery request and
new-attempt creation remain the existing workflow. This increment changes its
Core consumer and RPC response. The execution instructions require reading the
frozen PNG before editing and re-locating historical regions against current
content. Region numbers use the original array order and top-left normalized
coordinates. They do not assert Godot or Blender hits.

## Ownership and safety

- `object_attempt_callback.rs` validates tool, thread, turn and active lease,
  reads only the current attempt input, and rechecks cancellation after blob IO
  and image decoding. It now owns the complete tool-result envelope.
- `object_attempt_image.rs` decodes PNG with bounded dimensions/allocation and
  a 16 MP pixel limit. It returns source identity, hash, dimensions and byte
  count as text, and the exact original PNG bytes as a separate image item.
  The PNG base64 is not duplicated into the text item.
- `object_attempt_rpc.rs` forwards the envelope without wrapping it in text.
- The existing 1 MiB inline-file limit, snapshot membership and blob hash
  verification remain in force. The original project and mutable workspace
  are not image sources. Non-PNG formats retain their previous text response;
  capabilities explicitly advertise only PNG model-image delivery.
- Neither reading an image nor completing the model turn approves or publishes
  a result. Existing publication evidence and immutable feedback requests are
  not rewritten.

## Fresh verification

- `cargo test --locked -p beaver-core --lib object_candidate_rework`:
  **8 passed, 0 failed**, 1.04 s test runtime.
- `cargo test --locked -p beaver-core --lib object_attempt_callback`:
  **4 passed, 0 failed**, 1.40 s test runtime.
- Targeted `rustfmt --check --edition 2021`: passed.
- `npm run check:effective-lines`: **1147 sources, 17 unchanged legacy files,
  0 violations**. The baseline was not changed.

The new process-level RPC regression starts from a genuine candidate review
and image rework request, runs the normal attempt worker against a deterministic
app-server fixture, changes the workspace PNG, requests the original input,
and checks the exact image data URL and source metadata received by that
process. The worker then captures output and remains at the acceptance gate.
Adjacent tests cover durable reopen/retry and publication evidence; new tests
cover stale identities, bad hashes, corrupted blobs, cancellation, invalid PNG
decoding and a textual oversized-file refusal. The existing text callback
roundtrip also passes after the response-envelope change.

The first command used a nonexistent `native/Cargo.toml`; the workspace-root
command corrected it. The first new RPC test expected the medium task to stay
`running`; the actual established finish state is `awaitingAcceptance`. Only
that test expectation was corrected before the successful focused batch.
Three existing compiler warnings remain.

Logs: `output/image-delivery-rework.log`,
`output/image-delivery-callback.log`, `output/image-delivery-rustfmt.log`,
`output/image-delivery-structure.log`. Initial failure:
`output/image-delivery-core.log`.

## Remaining boundaries

This proves delivery over Beaver's real RPC transport to a deterministic
app-server fixture, not perception or editing quality by an external model
provider. No native UI acceptance or live provider call was run. UI and Desktop
APIs were unchanged; their prior focused evidence remains applicable.

Saved live-preview archives still cannot be submitted as task image feedback.
They must retain their archive provenance and cannot be represented as attempt
checkpoint files. Published/deferred followup image delivery, Blender preview,
engine hit selection and remaining F8/F9 acceptance are also open. F8.1-F8.5
remain unchecked; this is one completed consumer integration, not framework
completion.
