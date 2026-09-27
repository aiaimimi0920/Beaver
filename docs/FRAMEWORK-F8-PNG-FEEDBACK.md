# F8 frozen PNG feedback to candidate rework

Date: 2026-09-25

## Delivered workflow

Candidate review supports selecting a frozen output PNG, drawing up to eight
numbered rectangles, and adding regional comments plus an overall instruction.
Changing or clearing the image clears its regions. Recovery inspection and
explicit cost confirmation precede the new fine-task attempt.

Persisted feedback preserves path, SHA-256, decoded dimensions and normalized
rectangles. Core verifies output membership, blob integrity and PNG decoding.
Limits are 1 MiB content, 16,777,216 pixels and 16,384 pixels per dimension.
Regions represent image coordinates without claiming engine hits.

Image evidence enters the new fine-task prompt and publication review. Pending
requests restore without launching. Identical completed requests replay without
another lease; changed payloads conflict. Text-only feedback remains supported.

## Verification

- Core rework/image regressions: 6 passed; publication: 11 passed.
- Desktop image-feedback dispatch: 1 passed.
- Frontend rework/image/publication: 9 passed; geometry and stale selection: 2 passed.
- Typecheck, changed-file formatting and Rustfmt passed.
- Effective-line check: 1103 sources, 17 unchanged legacy files, 0 violations.

Logs: `output/f8-image-feedback-*.log`. Cargo stderr has PowerShell wrappers;
test result lines report the passing counts above. Existing warnings remain.

Chromium exercised production React components with mocked API transport: PNG
decode, pointer drag, regional comment, cost confirmation and submission passed.
The request preserved image identity and rectangle `(0.1, 0.2, 0.5, 0.5)` on a
400 x 200 image. Clearing the image removed its regions. Screenshot:
`output/playwright/f8-image-feedback/confirmation.png`. The console contained
only a missing fixture favicon and the React devtools notice. Windows multiline
CLI routing initially failed; `run-code --filename` with the local `smoke.js` passed.

## Remaining scope

Native WebView and real model execution were not exercised. Engine rendering,
camera controls, engine picking, provenance thumbnails and later-medium-task
feedback routing remain open. F8.1-F8.5 remain unchecked. No release, version
bump, commit or push was performed.
