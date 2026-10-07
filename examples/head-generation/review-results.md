# Candidate 177 review and verification

## Scope and generation

Original semantic recipe calibrated from the owner's candidate 162 review. Existing Beaver generation and actual MiDot/NPR comparison were used. The three permitted eye-adjacent accents, ear junction/detail, mouth and chin were revised. The accepted eye aperture, pupil, iris, catchlight, independent brow, body and hair were preserved.

- 86 pure recipe tests pass; 168 calibration keys all have consumers.
- Generation completion: f48d9fd1-4562-4c94-a340-0548a2833ce1, exit 0.
- Native validation: eda76db7-c168-4041-9218-4ea5294caa7f, pass, engineErrors false.
- GLB SHA256: b631362304173f485367c8a2fe1280cbd1fbb88b01b609c31a9883fc26213044.
- 13 generated source/spec hashes match. 1,682 protected ocular unique positions and original iris pixels match 162 exactly. Three explicitly editable accent color slots are excluded from that preservation mask.
- Ear-region 584-edge audit has no same-direction shared edges and no edges with more than two incident faces.
- Front chin skin follows full mouth opening by 1.50001 mm. The oral bag moves separately.

## Actual visual checks

From front to both 90-degree sides, all 19 neutral views at exact 10-degree steps were captured and inspected with matched framing/light. White front, bilateral side, rear and rear-oblique views, and wire front/rear-oblique views were inspected. Half-mouth 0.51 and full-mouth 1.0, paired near framing 0.61, minimum framing 0.25, Shift-drag synchronized pan, reset and light sweeps were exercised.

The lower-ear black triangle, posterior chin knee, folded mouth-corner triangle and small oblique lip shadow spots were corrected in the sampled views. Candidate 173's ineffective local normal-weighting experiment was reverted. Candidate 177 restores regular deeper oral-ring sampling after the mouth-inlet correspondence fix. Actual curved lip relief and original ear pigment/return improved. These observations do not establish identical likeness or final user aesthetic approval.

## Integration

Model-step aggregate run 420361c6-abfb-4737-8c72-a0d1d6b2256a: autoPassed, finished 2026-10-07T08:59:59.304644546+00:00. JUnit totals: {"name": "GutTests", "failures": "0", "tests": "9"}.

The initial aggregate 69eb4d3b-ec37-4017-96e5-862eb37a8587 failed during cold import before GUT: an imported scene could not be created/saved while storage was nearly exhausted; exit also reported dummy-renderer resource leaks. Completed historical workspace copies were archived losslessly, each file's SHA256 verified before its duplicate bytes were removed. Original paths can be restored from their archives. No test/assertion was removed or skipped. A real rerun determines the result above; failed evidence remains preserved.

Parent aggregate, final remote source verification and Library backup are tracked separately; this report does not claim those have finished yet. GitHub-hosted checks on prior source 8ca4ecbe8db831f1b33829d937c3dc13c1b41123 failed before runner/test steps. No CI success, merge, release, production deployment, full-character or wardrobe acceptance is claimed.

## Review handoff

Leave candidate 177 in the live comparison viewer. Use the near-framing slider, Shift-drag to pan both models, exact left/right 10-degree controls, white and wire modes. Owner manual aesthetic confirmation remains pending. The general customizable face template is a later step after the baseline appearance is accepted.
