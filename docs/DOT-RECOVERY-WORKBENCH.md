# Direct dot recovery workbench and source ownership

## Checkpoint scope (2026-10-05)

`src/direct/` preserves the Electron recovery workbench used with Beaver's
existing headless external-agent API. It provides task context, roles, explicit
tool calls, receipts, and an owned MiDot face-review process. It is a development
recovery UI, not the normal packaged Beaver desktop and not a model service.
The bridge source lives in `integrations/dot-bridge/README.md`.

The workbench was added alongside pinned Beaver source
`3eb6c9ad5744cd2ed0d925946d3933a26a463efc`; all locally recovered tracked files
remain byte-identical to that source. Missing files in the partial recovery
checkout are not deletions and must never be published as deletions.
Current main contains that commit and later advisory CI changes, which this
checkpoint preserves unchanged.

## Running the recovery UI

Install Beaver's existing npm dependencies and invoke Electron with
`src/direct/main.cjs`. This checkpoint retains the tested sibling layout:

- Parent of the Beaver checkout: `restored-tools/beaver/beaver-headless`
- `restored-tools/midot/midot-linux-x86_64/godot.linuxbsd.editor.x86_64`
- `beaver-restored-host/`, `beaver-restored-gui/`, `restored-projects/`
- `recovery-status/` for preview logs
- Optional explicitly approved input `recovered-library/Aster-NPR-lab-source.zip`
  for initial restoration; never use it over an existing working project

These artifacts and credentials are not shipped in Git. The preview deliberately
pins MiDot SHA-256
`888d345cda807f7dc6fae1a9aad6fbd256736e086a34d37819a39c19d311c5dd`;
a different engine is refused, not silently trusted. Linux Blender is expected
at `/usr/bin/blender`. Use a dedicated recovery directory, not a production
workspace. The initial restore helper is only for an approved archive and a
new empty project with no existing task history.

The Aster-specific preview expects the project-owned
`showcase/aster_head_review.tscn` and
`assets/aster/head_recovery_N/aster_definition.tres`. These project assets are
separate from Beaver source. Closing the UI shuts down the owned host; resuming
a waiting child can require continuing its existing paused parent plan first.
Use ordinary APIs, not task-database edits.

## Ownership audit

- Beaver: new recovery UI and dot bridge belong in this repository.
- MiDot: this round used the existing pinned executable; no engine-source edit
  was made. There is no new engine patch to submit from this round.
- NPRCharacterFrame / bundled NPR: the installed 344 files exactly match the
  existing Beaver package's pinned SHA-256 manifest. The package identifies
  upstream commit `a08b47a6cc229b6978afda26d74d13af690a67ba`. Do not submit
  generated project assets or differences against an unrelated archive as
  framework patches. No installed framework patch remains in this checkpoint.
- Aster face recipes, review scenes and model assets are project-owned and still
  in development. Candidate41 is not a finished Silver Wolf-like face or a
  validated general face template.

Future actual edits to MiDot or NPRCharacterFrame must be committed to their
respective GitHub repositories, with the Beaver pin updated when appropriate.
Do not count a private hosted Site commit as a Beaver repository commit.

## Verification

`node --test src/direct/preview.test.cjs` covers workspace/definition boundary,
symlink escape, wrong engine, duplicate launch, and idle close/status.
The existing live workbench has additionally generated and opened candidates
39–41; aesthetic acceptance remains pending. No new native app release is part
of this checkpoint.

Run repository formatting and `npm run check:effective-lines` before delivery.
If Rust is unavailable, report that checker as blocked; do not adjust its
baseline or claim a full native build. The added files can separately be
checked against the 500-line conservative physical-line limit.

Checkpoint verification results:

- Preview regression suite: 7 passed.
- Offline protocol suite: 8 passed (simulated callback, not live dot).
- Parameterized outbox transport preflight suite: 5 passed, no network calls.
- Formatted hosted overlay in the official starter: TypeScript check and
  production build passed; nothing redeployed as part of repository publication.
- Prettier check passed. Largest added source: 294 physical lines, below the
  500-line conservative bound without changing the repository baseline.
- Required `npm run check:effective-lines` attempted but blocked because `cargo`
  is unavailable in this recovery environment. Native build was not rerun.
