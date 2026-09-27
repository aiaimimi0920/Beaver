import assert from "node:assert/strict";
import test from "node:test";
import {
  execution,
  interruptReceipt,
  session,
} from "./fixtures/object-attempts";
import { verifiedView } from "./fixtures/object-recovery";
import { objectRecoveryResumeRequestSchema } from "../src/shared/object-recovery";

test("a new attempt replaces the old interrupt receipt and can be interrupted independently", async () => {
  let current = execution();
  let ids = 0;
  const targets: string[] = [];
  const view = { ...verifiedView(), canResume: true };
  const controller = session(
    async (method, input) => {
      if (method === "objectTask.attempts") return [structuredClone(current)];
      if (method === "objectTask.interrupt") {
        const receipt = interruptReceipt(input);
        targets.push(receipt.request.target.attemptId);
        current = {
          ...current,
          attempt: receipt.result,
          availability: "finished",
          checkpoints: { input: {}, output: {} },
        };
        return receipt;
      }
      if (method === "objectTask.resumeRecovery") {
        current = execution();
        current.attempt.target.attemptId = "attempt-2";
        return {
          schemaVersion: 1,
          request: objectRecoveryResumeRequestSchema.parse(input),
          result: {
            status: "started",
            report: view.operation!.result,
            attemptId: "attempt-2",
            fineTaskId: "fine",
          },
        };
      }
      if (method === "objectTask.recovery") return view;
      throw new Error(method);
    },
    async () => true,
    "p",
    () => "request-" + ++ids,
  );
  await controller.refresh();
  await controller.interrupt("attempt-1");
  assert.equal(
    controller.getSnapshot().receipt?.result.target.attemptId,
    "attempt-1",
  );
  await controller.recovery.refresh();
  assert.equal(controller.recovery.resume.prepare(), true);
  await controller.recovery.resume.submit();
  assert.equal(controller.getSnapshot().receipt, null);
  await controller.refresh();
  await controller.interrupt("attempt-2");
  assert.deepEqual(targets, ["attempt-1", "attempt-2"]);
});
