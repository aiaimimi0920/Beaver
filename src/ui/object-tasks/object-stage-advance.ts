import type { ObjectExecution } from "../../shared/object-attempts";
import type { ObjectTaskExecution } from "./object-task-execution";

export function successorFor(
  session: ObjectTaskExecution,
  attempt: ObjectExecution["attempt"],
) {
  const fines = session.fineTasks
    .filter((fine) => fine.status !== "cancelled")
    .slice()
    .sort(
      (a, b) =>
        (a.position ?? 0) - (b.position ?? 0) ||
        (a.id < b.id ? -1 : a.id > b.id ? 1 : 0),
    );
  const index = fines.findIndex(
    (fine) => fine.id === attempt.target.fineTaskId,
  );
  return index < 0 ? undefined : fines[index + 1];
}

export function prepareStageAdvance(
  session: ObjectTaskExecution,
  attempt: ObjectExecution["attempt"],
  note: string,
): boolean {
  const view = session.recovery.getSnapshot().view;
  const checks = session.checksFor(attempt).getSnapshot();
  const report = checks.reports.at(-1);
  const next = successorFor(session, attempt);
  if (
    attempt.state !== "awaitingGate" ||
    attempt.taskRevision !== view?.target.taskRevision ||
    !view.canDispose ||
    checks.busy ||
    checks.retry ||
    !report?.passed ||
    !next ||
    next.status !== "planned"
  )
    return false;
  return session.recovery.resume.prepareAdvance({
    attemptId: attempt.target.attemptId,
    checkRequestId: report.request.requestId,
    nextFineTaskId: next.id,
    nextFineRevision: next.revision,
    acceptanceNote: note,
  });
}
