import {
  objectRecoveryRequestSchema,
  type ObjectRecoveryOperation,
  type ObjectRecoveryRequest,
  type ObjectRecoveryTarget,
  type ObjectRecoveryView,
} from "../../src/shared/object-recovery";
import { ObjectTaskRecovery } from "../../src/ui/object-tasks/object-task-recovery";

export function recoveryTarget(): ObjectRecoveryTarget {
  return {
    taskId: "medium",
    objectId: "hero",
    runId: "run-hero",
    owner: "worker",
    claimToken: "claim-1",
    writerGeneration: 1,
    taskRevision: 3,
    runRevision: 2,
    objectRevision: 1,
    controlRevision: 0,
    recoveryGeneration: 0,
  };
}

export function recoveryRequest(
  requestId = "verify-1",
  target = recoveryTarget(),
): ObjectRecoveryRequest {
  return { projectId: "p", requestId, target };
}

export function recoveryView(): ObjectRecoveryView {
  return {
    target: recoveryTarget(),
    preparationState: "ready",
    paused: false,
    operation: null,
    disposition: null,
    resume: null,
    canResume: false,
    reportMatchesRecords: false,
    canDispose: false,
  };
}

export function recoveryReceipt(
  input: unknown = recoveryRequest(),
): ObjectRecoveryOperation {
  const request = objectRecoveryRequestSchema.parse(input);
  return {
    schemaVersion: 1,
    request,
    generation: request.target.recoveryGeneration + 1,
    result: {
      recordsCurrent: true,
      writerStatus: "stopRecorded",
      contentStatus: "verified",
      workspaceStatus: "matchesCheckpoint",
      paused: false,
      issues: [],
    },
  };
}

export function verifiedView(receipt = recoveryReceipt()): ObjectRecoveryView {
  return {
    ...recoveryView(),
    target: {
      ...receipt.request.target,
      recoveryGeneration: receipt.generation,
    },
    operation: structuredClone(receipt),
    reportMatchesRecords: true,
    canDispose: true,
  };
}

export function pendingView(
  request = recoveryRequest("saved-request"),
): ObjectRecoveryView {
  const operation = recoveryReceipt(request);
  operation.result = null;
  return {
    ...verifiedView(operation),
    reportMatchesRecords: false,
    canDispose: false,
  };
}

export function recoverySession(
  api: (method: string, input: unknown) => Promise<unknown>,
  requestId = () => "verify-1",
  projectId = "p",
): ObjectTaskRecovery {
  return new ObjectTaskRecovery(
    projectId,
    "medium",
    "hero",
    "run-hero",
    api,
    requestId,
  );
}
