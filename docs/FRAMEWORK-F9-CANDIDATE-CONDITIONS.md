# F9: candidate review conditions and publication

## Delivered workflow

New whole-object candidate reviews use report schema version 2. They no longer
claim that feedback disposition or managed publication is unimplemented. They
retain the final-fine owner acceptance requirement and explicitly direct the
owner to publication preview, file ownership review, feedback disposition and
the final human decision. Technical failure, baseline drift, reference closure
and file ownership findings remain unchanged.

The production review panel shows these conditions alongside the existing
publication controls. Historical version 1 reports are still displayed, with
their old capability limitations explicitly labeled as historical. Opening a
review does not approve, publish, rewrite evidence or bypass current validation.

## Persistence and compatibility

`object_candidate_summary::build` reconstructs each saved report according to
its stored schema version. `checked` still compares the complete reconstruction
with the persisted report. New requests produce v2; exact retries of v1 return
the original v1 bytes/values rather than silently upgrading them. Source and
output digests, stored source shape and publication receipt schema are unchanged.

The TypeScript reader accepts v1 and v2, enforces the matching condition set and
rejects unknown versions or crossed old/new condition sets. The existing Desktop
API transports the versioned report unchanged. Publication and rework continue
to verify frozen records and current state using the existing Core paths.

No data migration or deletion runs. Existing v1 records remain readable and
publishable by the updated code. Older binaries do not understand newly written
v2 records; downgrading a project with v2 evidence is not a supported rollback.
Preserve the prior project backup if binary rollback is required; do not relabel
or rewrite persisted v2 records to make an old reader accept them.

## Focused verification (2026-09-25)

The final focused commands all passed:

```text
cargo fmt --all
npx prettier --write src/shared/object-candidate-review.ts src/ui/object-tasks/ObjectCandidateReviewPanel.tsx tests/object-candidate-review.test.ts
npm run typecheck
npx tsx --test tests/object-candidate-review.test.ts tests/object-candidate-rework.test.ts tests/object-publication.test.ts tests/object-check-report.test.ts
cargo test --locked -p beaver-core --lib object_candidate -- --nocapture
cargo test --locked -p beaver-desktop candidate_review_routes -- --nocapture
npm run check:effective-lines
git diff --check
```

- Frontend: 16 passed. Covers current and historical review rendering with
  publication controls, unknown responses, exact retry, malformed evidence,
  report navigation and adjacent publication/rework behavior.
- Core: 7 passed. Includes v1/v2 project reopen, exact review replay, publication,
  repeated publication, unchanged historical evidence after publication, and
  refusal of unknown versions or modified conditions before any file write.
- Desktop: 1 passed, exercising the actual candidate-review route and replay.
- Structure: 1089 sources, 17 unchanged legacy files, 0 violations.
- Independent read-only review found no concrete persistence or retry defect.

Logs: `output/f9-review-{format,types,tests,core-fixed,desktop-routes,lines-fixed}.log`.
The initial Core compile found a test-only mutable-reference serialization borrow
error; it was fixed and the focused Core suite rerun. The first Desktop filter
matched zero tests; the corrected `candidate_review_routes` filter ran one test.
Neither initial command is counted as functional evidence.

## Remaining plan scope

This closes an obsolete production-state message and its versioned persistence
contract. It does not finish the F9 inventory, native UI acceptance, configurable
stage policies or AI evaluation. F6.6, F7.1 and F9 remain unchecked. No native
release, full end-to-end acceptance or version bump was performed.
