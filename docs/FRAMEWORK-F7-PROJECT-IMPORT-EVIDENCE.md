# F7.4 project object import evidence

Date: 2026-09-25

## Delivered workflow

Registered and external Beaver project preparations now support explicit formal
import in the production import-history dialog. Together with ordinary files
and folders, all three sources use the same durable file journal and final
catalog transaction. Users can reopen a saved preparation, confirm import,
query or retry the original operation, abort incomplete writes, and open an
imported object from the result.

The source adapter copies the selected accepted version and its fixed dependency
closure into target content-addressed storage before starting the journal.
The deterministic preparation identity map creates new object, component and
version IDs. All pinned versions and blobs are retained; every imported version
is marked importedPendingValidation. Per-object provenance links to the saved
preparation, source project/object, and closure digest. Ordinary accepted-reference
validation remains unchanged.

Project-relative asset paths and bytes are preserved. The selected root version
is the root working projection; dependencies use their first frozen version.
Other historical versions remain available as immutable manifests and blobs.
Organizational parents outside the content closure are omitted; internal parents
are remapped and cycles rejected.

## Recovery and safety

- Source snapshot and content hashes are checked before freezing; copied hashes
  and source blobs are checked again. Sources are never rewritten or deleted.
- After journal creation, query, retry and abort require only target-local saved
  preparation, journal and frozen blobs. Source paths may be offline.
- Write intents precede file writes. Existing target paths are conflicts, even
  if their bytes match but no journal intent owns them.
- Abort removes only attributable matching writes. External edits remain and
  block cleanup until explicitly resolved.
- Objects, every version, provenance and completed operation commit together.
  Injected transaction failure leaves no partially registered catalog.
- Pending versions cannot be used as ordinary accepted references. References
  within the imported closure remain pinned to the remapped pending versions.

## Verification

Native commands ran in the normal PowerShell toolchain environment, through
rtk proxy. Cargo test harness summaries below are the result evidence; PowerShell
also rendered Cargo stderr warnings as NativeCommandError and the wrapper
sometimes returned 1 despite successful harness results. Existing unrelated
unused/dead-code warnings remain.

- cargo test --locked -p beaver-core --lib object_file_import -j 1:
  15 passed. Includes file preparations, file import and project import.
- cargo test --locked -p beaver-core --lib object_import_preparation -j 1:
  25 passed. Covers source identity, closure validation and source isolation.
- After strengthening source-preservation assertions:
  cargo test --locked -p beaver-core --lib object_file_import::project_tests -j 1:
  4 passed. Registered and external sources; two versions of one dependency;
  original asset paths; source blob bytes/mtime and fresh source catalog digest;
  offline runtime reopen at prepared/intent/written/beforeCommit boundaries;
  conflicts, external edits during abort, corrupted source/receipt, atomic rollback.
- cargo test --locked -p beaver-desktop data_dispatch_object_import -j 1:
  9 passed. Public catalog validation and dispatch, target isolation, offline
  replay/query and refusal to abort a completed import.
  An initial fixture used internal sourcePath/sourceProjectId fields; corrected
  to the public source object and reran this suite. An earlier filename-based
  filter selected zero tests and is not counted as verification.
- npx tsx --test tests/object-file-import.test.ts tests/object-import-history.test.ts:
  11 passed. Project routing, stable preparation identity, reopen/retry/abort,
  pending status, and project confirmation content.
- npm run typecheck: passed.
- Focused Prettier and rustfmt checks: passed.
- npm run check:effective-lines: 1034 sources, 0 violations; no baseline changes.
- git diff --check: passed; existing Git CRLF conversion warnings only.

Logs: output/project-import-core.log, output/project-import-core-final.log,
output/project-import-preparation.log, output/project-import-desktop-final.log,
output/project-import-ui.log, output/project-import-typecheck.log,
output/project-import-structure.log.

## Boundaries and remaining work

F7.4 is complete at the development slice boundary. F7.5 remains open for the
full generation-dialog workflow. F7.1 and F7.3 retain their existing plan gaps.
Promotion of imported pending versions to accepted versions is not implemented
by this slice. No Godot validation, native GUI acceptance, release packaging,
deployment, commit, push or version increment was performed.

Imports intentionally reject colliding asset paths; there is no automatic
path rewriting. Limits are 512 MiB of unique content and 1024 objects per import.
Recovery relies on the durable target preparation as well as the journal;
arbitrary deletion or corruption of those records is rejected, not repaired.

A read-only review was checked against the implementation: external parents are
cleared on every manifest before parent-chain validation; the reconstructed
source closure validates pinned references before remapping; journal writes must
be a prefix of the unique planned paths; captured content must match its expected
hash and source blobs are reverified. These checks did not justify the review's
suggested global catalog-validation relaxation or duplicate recovery machinery.

