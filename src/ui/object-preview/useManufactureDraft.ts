import { useState } from "react";
import type { DemoObject } from "./mock-objects";
import type {
  PreviewAnnotation,
  PreviewFeedback,
  PreviewMode,
} from "./preview-target";
import { manufactureViews } from "./manufacture-preview";

export interface ManufacturePrompts {
  creation: string;
  acceptance: string;
  output: string;
}
export interface ManufactureChecks {
  geometry: boolean;
  views: boolean;
  visual: boolean;
  triangleLimit: string;
}
export interface ManufactureDraft {
  prompts: ManufacturePrompts;
  checks: ManufactureChecks;
  instruction: string;
  annotations: PreviewAnnotation[];
  nextNumber: number;
}
export type RegenerateScope = "stage" | "downstream";
export interface ManufactureRound {
  number: number;
  source?: number;
  draft: ManufactureDraft;
  scope: RegenerateScope;
}
interface StageSession {
  draft: ManufactureDraft;
  rounds: ManufactureRound[];
  viewed: number;
  mode: PreviewMode | null;
  notice: string;
}

function initialDraft(
  object: DemoObject,
  version: string,
  title: string,
  taskTitle?: string,
): ManufactureDraft {
  return {
    prompts: {
      creation: `以「${object.name}」的 ${version} 版本为基准，完成「${title}」阶段。\n${taskTitle ?? object.description}。保留已有设计中不涉及本次修改的部分。`,
      acceptance: `核对「${title}」的创建要求，结合工具报告与效果图检查轮廓、细节和视觉表现。\n逐项说明依据；不满足要求时标出位置，给出修改建议。`,
      output: `验收数据：${manufactureViews(object).join("、")}及检查报告。\n输出数据：${object.components.map((part) => `${part.name}（${part.format}）`).join("、")}。\n保留可编辑源文件与相对引用。`,
    },
    checks: {
      geometry: object.objectType === "模型",
      views: true,
      visual: true,
      triangleLimit: "40000",
    },
    instruction: "",
    annotations: [],
    nextNumber: 1,
  };
}

export function useManufactureDraft(
  object: DemoObject,
  version: string,
  stage: number,
  title: string,
  pending: boolean,
  taskTitle?: string,
) {
  const [sessions, setSessions] = useState<Record<string, StageSession>>({});
  const key = `${object.id}:${version}:${stage}`;
  const draft = initialDraft(object, version, title, taskTitle);
  const initial: StageSession = {
    draft,
    rounds: pending ? [] : [{ number: 1, draft, scope: "stage" }],
    viewed: pending ? 0 : 1,
    mode: null,
    notice: "",
  };
  const session = sessions[key] ?? initial;
  const update = (fn: (value: StageSession) => StageSession) =>
    setSessions((previous) => ({
      ...previous,
      [key]: fn(previous[key] ?? initial),
    }));
  const edit = (patch: Partial<ManufactureDraft>) =>
    update((value) => ({
      ...value,
      draft: { ...value.draft, ...patch },
      notice: "草稿已修改 · 尚未重新生成",
    }));
  const round = session.rounds.find((value) => value.number === session.viewed);

  return {
    ...session,
    round,
    edit,
    view: (viewed: number) =>
      update((value) => ({
        ...value,
        viewed,
        mode: null,
        notice: "仅切换结果轮次，当前提示词草稿保留",
      })),
    toggleMode: (mode: PreviewMode) =>
      update((value) => ({
        ...value,
        mode: value.mode === mode ? null : mode,
      })),
    add: ({ target, selection }: PreviewFeedback) => {
      if (selection.kind === "whole") return;
      update((value) =>
        !value.mode
          ? value
          : {
              ...value,
              draft: {
                ...value.draft,
                nextNumber: value.draft.nextNumber + 1,
                annotations: [
                  ...value.draft.annotations,
                  {
                    number: value.draft.nextNumber,
                    target,
                    selection,
                    prompt: "",
                  },
                ],
              },
              notice: "位置已加入补充修改，可按编号描述要求",
            },
      );
    },
    save: () =>
      update((value) => ({
        ...value,
        notice: "已保留本阶段草稿（本次 UI 会话）",
      })),
    restore: () => {
      if (round)
        update((value) => ({
          ...value,
          draft: round.draft,
          mode: null,
          notice: `已载入第 ${round.number} 轮的提示词与标注，可编辑后重新生成`,
        }));
    },
    generate: (scope: RegenerateScope) =>
      update((value) => {
        const number = (value.rounds.at(-1)?.number ?? 0) + 1;
        return {
          ...value,
          rounds: [
            ...value.rounds,
            {
              number,
              source: value.viewed || undefined,
              draft: value.draft,
              scope,
            },
          ],
          viewed: number,
          draft: {
            ...value.draft,
            instruction: "",
            annotations: [],
            nextNumber: 1,
          },
          mode: null,
          notice: `已创建第 ${number} 轮 Mock 结果，保留旧轮次；${scope === "stage" ? "后续阶段标记为待更新" : "后续阶段已列入重做范围"}`,
        };
      }),
  };
}

export type ManufactureSession = ReturnType<typeof useManufactureDraft>;
