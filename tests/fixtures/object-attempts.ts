import {
  objectAttemptInterruptSchema,
  type ObjectAttemptInterruptReceipt,
  type ObjectExecution,
} from "../../src/shared/object-attempts";
import { ObjectTaskExecution } from "../../src/ui/object-tasks/object-task-execution";
import { objectTaskSnapshot } from "./object-tasks";

export function execution(
  state: ObjectExecution["attempt"]["state"] = "running",
  availability: ObjectExecution["availability"] = state === "running"
    ? "active"
    : "finished",
): ObjectExecution {
  return {
    attempt: {
      projectId: "p",
      target: {
        taskId: "medium",
        fineTaskId: "fine",
        objectId: "hero",
        runId: "run-hero",
        attemptId: "attempt-1",
        owner: "worker",
        claimToken: "claim-1",
        generation: 1,
        threadId: null,
        turnId: null,
      },
      taskRevision: 2,
      state,
      outputCaptured: state !== "running",
      error: state === "failed" ? "OBJECT_ATTEMPT_LAUNCH_FAILED" : null,
    },
    availability,
    checkpoints: { input: {}, output: state === "running" ? null : {} },
    definition: {
      title: "Movement",
      prompt: "Implement movement",
      acceptance: "Movement works",
      revision: 1,
    },
  };
}

export function interruptReceipt(
  input: unknown,
  state: Exclude<
    ObjectExecution["attempt"]["state"],
    "running"
  > = "interrupted",
): ObjectAttemptInterruptReceipt {
  const request = objectAttemptInterruptSchema.parse(input);
  return {
    request,
    result: {
      ...execution(state).attempt,
      projectId: request.projectId,
      target: structuredClone(request.target),
      taskRevision: request.expectedTaskRevision + 1,
    },
  };
}

export function session(
  api: (method: string, input: unknown) => Promise<unknown>,
  refreshWorkspace = async () => true,
  projectId = "p",
  requestId = () => "stop-1",
): ObjectTaskExecution {
  return new ObjectTaskExecution(
    projectId,
    objectTaskSnapshot(projectId),
    "medium",
    api,
    refreshWorkspace,
    requestId,
  );
}

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
