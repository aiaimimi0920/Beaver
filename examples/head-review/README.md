# Native linked head comparison example

Project-owned MiDot/NPR review tooling, preserved alongside Beaver's direct recovery workflow. This is not a patch to the immutable `npr-characters` package or to the MiDot engine. It does not include character assets or claim artistic acceptance.

## Dependencies and integration

- A Beaver project with the pinned NPRCharacterFrame package (tested upstream provenance `a08b47a6cc22`, package NPR 1.3.0).
- Tested MiDot source `38b6ddee7`; runtime binary SHA-256 `888d345cda807f7dc6fae1a9aad6fbd256736e086a34d37819a39c19d311c5dd`.
- Project-owned `assets/aster/head_recovery_49/aster_definition.tres` and `assets/aster/head_recovery_900/aster_definition.tres`, or adapt these two explicit sample paths in `head_review.gd` to authorized local assets. Assets are deliberately not included.
- Copy `showcase_extensions/` and `showcase/aster_head_review.tscn` into the project through its normal Beaver file workflow. Read the scene's initial definition path and adapt it to your project before running. The recovery workbench overrides the preview definition with the selected path.
- Keep the addon package unchanged. The three runtime roles Body/Face/Hair must match the NPR definition contract.

## Controls

Open the head reviewer, then select `模型对比`. The current candidate is shown on the left and reference on the right. Opening from the reference page uses the explicit candidate49 fallback; this is not an automatic latest-candidate selector. The model label shows the actual candidate folder.

Both viewports share angle, zoom, material mode and NPR light yaw. Each face is normalized by its own neutral Face AABB height only for preview; source transforms/resources are not changed. Body and hair default hidden. Reference Head rotation is neutralized in the duplicate preview. Drag either face to rotate both; use the wheel to zoom. Render, white and runtime triangle wireframe modes are available. Runtime triangle edges do not prove the author's original quad layout.

F12 saves a real viewport PNG and adjacent JSON with definition paths and shared state under `artifacts/model_comparison/`. Wait for the saved status before changing the view. Escape or `返回审查` closes the overlay and restores the original reviewer.

## Verified checkpoint

Installed with Beaver job `992eafc7-dc81-4a99-a21d-18e558b45efe`, then opened in native MiDot on 2026-10-05. Verified candidate49/reference900 identities, equal-relative face height, shared zoom, front/oblique/profile controls, render/white/wire modes, reversed light, linked drag and close/reopen. Actual PNGs were inspected independently. No synthetic screenshots or browser substitute were used.

The candidate-only MouthOpen slider defaults to zero. Values 0, 0.5 and 1 were exercised against generated candidate50; the reference has no matching morph and deliberately stays neutral. White mode uses the real source mesh; the wire duplicate preserves blend-shape arrays and synchronizes weights. Neutral face framing uses base surface vertices, not the AABB expanded by morph motion. Reopened candidates51–53 confirmed morph discovery, default neutral state and normalized framing. This verifies the inspection tool, not artistic quality: oral rim artifacts and overall facial acceptance remain open.

## Ownership and publication

These scripts extend the consuming project's review scene. Beaver owns this reproducible example and its workflow; NPRCharacterFrame and MiDot remain unmodified. Do not copy private reference models, cloud credentials, task databases, or generated project caches into this repository.
