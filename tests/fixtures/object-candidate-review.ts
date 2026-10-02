import {
  objectCandidateRequestSchema,
  objectCandidateReportSchema,
} from "../../src/shared/object-candidate-review";

export function candidateReport(input: unknown, schemaVersion: 1 | 2 = 2) {
  return objectCandidateReportSchema.parse({
    schemaVersion,
    request: objectCandidateRequestSchema.parse(input),
    time: "2026-09-25T12:00:00Z",
    sourceDigest: "a".repeat(64),
    outputDigest: "b".repeat(64),
    baselineVersionId: "historic",
    acceptedVersionIdAtClaim: "current",
    acceptedVersionIdAtReview: "new-current",
    objectRevisionAtClaim: 1,
    objectRevisionAtReview: 3,
    stages: [
      {
        taskId: "fine",
        title: "Movement",
        revision: 3,
        status: "awaitingAcceptance",
        acceptance: "Movement works",
      },
    ],
    references: [{ objectId: "material", versionId: "material-v1" }],
    files: [
      {
        path: "hero.gd",
        before: "c".repeat(64),
        after: "d".repeat(64),
        owners: ["hero"],
        reference: false,
      },
    ],
    rules: ["checkpoint-integrity", "code-structure"].map((id) => ({
      id,
      version: 1,
      passed: true,
      issues: [],
      filesChecked: 1,
    })),
    blockers: [
      "FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED",
      ...(schemaVersion === 1
        ? ["FEEDBACK_REVIEW_UNAVAILABLE", "PUBLICATION_NOT_IMPLEMENTED"]
        : ["PUBLICATION_CONFIRMATION_REQUIRED"]),
      "ACCEPTED_VERSION_DRIFT",
    ],
  });
}
