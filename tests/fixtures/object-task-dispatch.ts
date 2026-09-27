import {
  setObjectTaskPausedSchema,
  type ObjectTaskDispatchControl,
  type ObjectTaskDispatchReceipt,
} from "../../src/shared/object-task-dispatch";
import { ObjectTaskDispatch } from "../../src/ui/object-tasks/object-task-dispatch";
import { objectTaskSnapshot } from "./object-tasks";

export function dispatchControl(
  paused = true,
  revision = 1,
): ObjectTaskDispatchControl {
  return {
    schemaVersion: 1,
    projectId: "p",
    taskId: "medium",
    objectId: "hero",
    runId: "run-hero",
    paused,
    revision,
  };
}

export function dispatchReceipt(input: unknown): ObjectTaskDispatchReceipt {
  const request = setObjectTaskPausedSchema.parse(input);
  return {
    request,
    result: {
      schemaVersion: 1,
      projectId: request.projectId,
      taskId: request.taskId,
      objectId: request.objectId,
      runId: request.runId,
      paused: request.paused,
      revision: request.expectedControlRevision + 1,
    },
  };
}

export function dispatchSession(
  api: (method: string, input: unknown) => Promise<unknown>,
  refresh: () => Promise<boolean> = async () => true,
  id: () => string = () => "pause-1",
) {
  return new ObjectTaskDispatch(
    "p",
    objectTaskSnapshot(),
    "medium",
    api,
    refresh,
    id,
  );
}

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
