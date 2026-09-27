# F8 candidate feedback draft recovery

Date: 2026-09-25

## Delivered workflow

The production candidate rework panel now retains unsent feedback for both
current rework and later medium-task routing. Text, title, acceptance criteria,
frozen PNG identity, normalized regions and region prompts survive a panel
reopen or page reload in the same WebView storage profile. Restore never submits.
Drafts are scoped to the complete project, target and review identity.

The editor reloads the original frozen PNG and validates its decoded dimensions
before enabling an image-bearing submission. Missing or changed source identity
does not silently select another image. Incomplete region prompts remain editable
draft data; the existing request validator still governs submission.

Storage failures retain edits in memory and offer retry. Corrupt or foreign
records block replacement and remain available for recovery. Publication conflicts
retain both editable input and the separately persisted immutable pending request;
retry cannot change that request's target or payload. Completed submissions retain
the local draft for reference. Storage is local to the WebView profile, without
cross-device or replacement-profile recovery.

## Verification

- Focused TypeScript tests: 14 passed, 0 failed across
  object-candidate-feedback-draft, object-candidate-rework,
  object-publication-deferred and object-publication. Coverage includes review
  isolation, incomplete Unicode input, write/read failures, corrupt/foreign data,
  publication conflict and immutable retries.
- Chromium used the production React components and sessions with a mocked API.
  It selected a PNG, drew a region, entered feedback and reloaded. All inputs and
  stored coordinates survived unchanged with no write API call on restore.
  Submission included the restored 400 x 200 image and exact region data. A lost
  response followed by reload retried the original request, recorded one save,
  and completed publication with a followup receipt. Screenshots were inspected.
- npm run typecheck: passed.
- Targeted Prettier check: passed for all five changed source/test files.
- npm run check:effective-lines: 1118 sources, 17 unchanged legacy files,
  0 violations. The existing baseline was not changed.
- git diff --check: passed; Git reported existing LF/CRLF conversion warnings.

Logs: output/f8-feedback-draft-tests.log,
output/f8-feedback-draft-browser-final.log,
output/f8-feedback-draft-typecheck-final.log,
output/f8-feedback-draft-format.log, output/f8-feedback-draft-lines.log.

Browser harness and screenshots:
output/playwright/f8-feedback-draft/{harness.tsx,smoke.js,restored.png,published.png}.
The dedicated browser session and HTTP server were closed afterward.

## Remaining boundary

This closes the candidate-form local draft workflow within F8.5. It does not
complete F8.1-F8.5: real Godot/Blender preview and selection, old-topology
relocalization, and native acceptance remain outstanding. The browser API was
mocked, so this evidence does not establish a new native end-to-end or release
acceptance result. No Core or Desktop behavior changed in this slice.
