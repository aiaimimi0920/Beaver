import { z } from "zod";
import { objectReworkImageSchema } from "./object-rework-image";
import { previewFrameReferenceSchema } from "./object-publication";
import { feedbackRelocationSchema } from "./preview-feedback-relocation";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const revision = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
const text = z.string().min(1).max(128);

export const objectRecoveryTargetSchema = z.strictObject({
  taskId: id,
  objectId: id,
  runId: id,
  owner: text.refine((value) => value.trim().length > 0),
  claimToken: text,
  writerGeneration: revision.min(1),
  taskRevision: revision,
  runRevision: revision,
  objectRevision: revision,
  controlRevision: revision,
  recoveryGeneration: revision,
});

export const objectRecoveryRequestSchema = z.strictObject({
  projectId: id,
  requestId: id,
  target: objectRecoveryTargetSchema.extend({
    recoveryGeneration: revision.max(Number.MAX_SAFE_INTEGER - 1),
  }),
});

export const objectRecoveryReportSchema = z.strictObject({
  recordsCurrent: z.boolean(),
  writerStatus: z.enum(["active", "stopRecorded", "unconfirmed"]),
  contentStatus: z.enum(["verified", "invalid", "unchecked"]),
  workspaceStatus: z.enum([
    "matchesCheckpoint",
    "drifted",
    "missing",
    "invalid",
    "unchecked",
  ]),
  paused: z.boolean(),
  issues: z.array(z.string()),
});

export const objectRecoveryOperationSchema = z
  .strictObject({
    schemaVersion: z.literal(1),
    request: objectRecoveryRequestSchema,
    generation: revision.min(1),
    result: objectRecoveryReportSchema.nullable(),
  })
  .refine(
    (operation) =>
      operation.generation === operation.request.target.recoveryGeneration + 1,
    { message: "核验代次与请求不一致" },
  );

function canDispose(report: z.infer<typeof objectRecoveryReportSchema>) {
  return (
    report.recordsCurrent &&
    report.writerStatus === "stopRecorded" &&
    report.contentStatus === "verified" &&
    report.workspaceStatus === "matchesCheckpoint" &&
    !report.paused &&
    report.issues.length === 0
  );
}

export const objectRecoveryDispositionRequestSchema = z.strictObject({
  projectId: id,
  requestId: id,
  target: objectRecoveryTargetSchema.extend({
    recoveryGeneration: revision.min(1),
  }),
  verificationRequestId: id,
  choice: z.enum(["cancelAndKeep", "cancelAndRemoveWorkspace"]),
});

export const objectRecoveryDispositionOperationSchema = z
  .strictObject({
    schemaVersion: z.literal(1),
    request: objectRecoveryDispositionRequestSchema,
    result: z
      .discriminatedUnion("status", [
        z.strictObject({
          status: z.literal("cancelledAndRetained"),
          report: objectRecoveryReportSchema,
          retained: z.strictObject({
            workspace: z.string().min(1),
            attemptId: id,
            fineTaskId: id,
            outputFileCount: revision,
          }),
          taskRevision: revision.min(1),
          runRevision: revision.min(1),
          planRevision: revision.min(1),
        }),
        z.strictObject({
          status: z.literal("cancelledAndWorkspaceRemoved"),
          report: objectRecoveryReportSchema,
          removed: z.strictObject({
            workspace: z.string().min(1),
            attemptId: id,
            fineTaskId: id,
            outputFileCount: revision,
          }),
          taskRevision: revision.min(1),
          runRevision: revision.min(1),
          planRevision: revision.min(1),
        }),
        z.strictObject({
          status: z.literal("blocked"),
          report: objectRecoveryReportSchema,
        }),
      ])
      .nullable(),
  })
  .refine(
    ({ request, result }) => {
      if (!result) return true;
      if (result.status === "blocked") return !canDispose(result.report);
      return (
        canDispose(result.report) &&
        (request.choice === "cancelAndRemoveWorkspace") ===
          (result.status === "cancelledAndWorkspaceRemoved") &&
        result.taskRevision === request.target.taskRevision + 1 &&
        result.runRevision === request.target.runRevision + 1
      );
    },
    { message: "处置回执与核验证据或任务版本不一致" },
  );

export const objectStageApprovalSchema = z.strictObject({
  attemptId: id,
  checkRequestId: id,
  nextFineTaskId: id,
  nextFineRevision: revision.max(Number.MAX_SAFE_INTEGER - 1),
  acceptanceNote: z
    .string()
    .refine((value) => value.trim().length > 0)
    .refine((value) => new TextEncoder().encode(value).length <= 4000),
});
export type ObjectStageApproval = z.infer<typeof objectStageApprovalSchema>;
export const objectCandidateReworkSchema = z
  .strictObject({
    reviewRequestId: id,
    attemptId: id,
    fineTaskId: id,
    image: objectReworkImageSchema.optional(),
    previewFrame: previewFrameReferenceSchema.optional(),
    relocation: feedbackRelocationSchema.optional(),
    feedback: z
      .string()
      .refine((value) => value.trim().length > 0)
      .refine((value) => new TextEncoder().encode(value).length <= 4000),
  })
  .refine(
    (value) =>
      !(value.image && value.previewFrame) &&
      (!value.relocation || !!value.previewFrame),
    {
      message: "图片与编号存档不能同时附加",
    },
  );
export type ObjectCandidateRework = z.infer<typeof objectCandidateReworkSchema>;
export const objectRecoveryResumeRequestSchema =
  objectRecoveryDispositionRequestSchema
    .omit({ choice: true })
    .extend({
      advance: objectStageApprovalSchema.optional(),
      rework: objectCandidateReworkSchema.optional(),
    })
    .refine((request) => !(request.advance && request.rework), {
      message: "不能同时推进阶段和返工候选",
    });
export const objectRecoveryResumeOperationSchema = z
  .strictObject({
    schemaVersion: z.literal(1),
    request: objectRecoveryResumeRequestSchema,
    result: z
      .discriminatedUnion("status", [
        z.strictObject({
          status: z.literal("started"),
          report: objectRecoveryReportSchema,
          attemptId: id,
          fineTaskId: id,
        }),
        z.strictObject({
          status: z.literal("blocked"),
          report: objectRecoveryReportSchema,
        }),
      ])
      .nullable(),
  })
  .refine(
    ({ request, result }) =>
      !result ||
      ((result.status === "started") === canDispose(result.report) &&
        (result.status !== "started" ||
          !request.advance ||
          result.fineTaskId === request.advance.nextFineTaskId) &&
        (result.status !== "started" ||
          !request.rework ||
          (result.fineTaskId === request.rework.fineTaskId &&
            result.attemptId !== request.rework.attemptId))),
    { message: "重试回执与证据不一致" },
  );
export type ObjectRecoveryResumeRequest = z.infer<
  typeof objectRecoveryResumeRequestSchema
>;
export type ObjectRecoveryResumeOperation = z.infer<
  typeof objectRecoveryResumeOperationSchema
>;

export const objectRecoveryViewSchema = z
  .strictObject({
    target: objectRecoveryTargetSchema,
    preparationState: z.enum(["pending", "preparing", "ready", "failed"]),
    paused: z.boolean(),
    operation: objectRecoveryOperationSchema.nullable(),
    disposition: objectRecoveryDispositionOperationSchema.nullable(),
    resume: objectRecoveryResumeOperationSchema.nullable(),
    canResume: z.boolean(),
    reportMatchesRecords: z.boolean(),
    canDispose: z.boolean(),
  })
  .refine(
    (view) => {
      const operation = view.operation;
      if (!operation)
        return (
          view.target.recoveryGeneration === 0 &&
          !view.disposition &&
          !view.reportMatchesRecords &&
          !view.canDispose &&
          !view.canResume &&
          !view.resume
        );
      const target = operation.request.target;
      if (
        view.target.recoveryGeneration !== operation.generation ||
        view.target.taskId !== target.taskId ||
        view.target.runId !== target.runId ||
        view.target.objectId !== target.objectId
      )
        return false;
      const report = operation.result;
      const resume = view.resume;
      if (
        resume &&
        (resume.request.projectId !== operation.request.projectId ||
          resume.request.target.taskId !== target.taskId ||
          resume.request.target.runId !== target.runId ||
          resume.request.target.objectId !== target.objectId ||
          resume.request.target.recoveryGeneration > operation.generation ||
          (resume.request.target.recoveryGeneration === operation.generation &&
            (resume.request.verificationRequestId !==
              operation.request.requestId ||
              !report ||
              !canDispose(report) ||
              JSON.stringify(resume.request.target) !==
                JSON.stringify({
                  ...target,
                  recoveryGeneration: operation.generation,
                }))) ||
          (!resume.result &&
            resume.request.target.recoveryGeneration !== operation.generation))
      )
        return false;
      if (
        view.canResume &&
        (!view.canDispose ||
          (resume &&
            resume.request.target.recoveryGeneration >= operation.generation))
      )
        return false;
      const disposition = view.disposition;
      if (disposition) {
        const request = disposition.request;
        const generation = request.target.recoveryGeneration;
        if (
          request.projectId !== operation.request.projectId ||
          request.target.taskId !== target.taskId ||
          request.target.runId !== target.runId ||
          request.target.objectId !== target.objectId ||
          generation > operation.generation ||
          (generation < operation.generation &&
            disposition.result?.status !== "blocked") ||
          (generation === operation.generation &&
            (!report ||
              !canDispose(report) ||
              request.verificationRequestId !== operation.request.requestId ||
              JSON.stringify(request.target) !==
                JSON.stringify({
                  ...target,
                  recoveryGeneration: generation,
                }))) ||
          ((disposition.result?.status === "cancelledAndRetained" ||
            disposition.result?.status === "cancelledAndWorkspaceRemoved") &&
            view.reportMatchesRecords)
        )
          return false;
      }
      if (
        view.reportMatchesRecords &&
        (!report ||
          report.paused !== view.paused ||
          JSON.stringify(view.target) !==
            JSON.stringify({
              ...target,
              recoveryGeneration: operation.generation,
            }))
      )
        return false;
      return (
        view.canDispose ===
        (view.reportMatchesRecords &&
          (!resume || !!resume.result) &&
          (!disposition ||
            (disposition.result?.status === "blocked" &&
              disposition.request.target.recoveryGeneration <
                operation.generation)) &&
          !!report &&
          canDispose(report))
      );
    },
    { message: "核验报告与当前记录或处置资格不一致" },
  );

export type ObjectRecoveryTarget = z.infer<typeof objectRecoveryTargetSchema>;
export type ObjectRecoveryRequest = z.infer<typeof objectRecoveryRequestSchema>;
export type ObjectRecoveryOperation = z.infer<
  typeof objectRecoveryOperationSchema
>;
export type ObjectRecoveryView = z.infer<typeof objectRecoveryViewSchema>;
export type ObjectRecoveryDispositionRequest = z.infer<
  typeof objectRecoveryDispositionRequestSchema
>;
export type ObjectRecoveryDispositionOperation = z.infer<
  typeof objectRecoveryDispositionOperationSchema
>;
