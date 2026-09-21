import { useState } from "react";
import type { DemoTask } from "./mock-tasks";
import { suggestMockIterationTitle } from "./mock-iterations";
import type {
  PreviewAnnotation,
  PreviewFeedback,
  PreviewMode,
} from "./preview-target";

interface IterationDraft {
  objectId: string;
  afterId: string | null;
  taskId: string | null;
  title: string;
  prompt: string;
  mode: PreviewMode | null;
  annotations: PreviewAnnotation[];
  nextNumber: number;
}

export function useIterationDraft() {
  const [draft, setDraft] = useState<IterationDraft | null>(null);
  const begin = (objectId: string, afterId: string | null) =>
    setDraft({
      objectId,
      afterId,
      taskId: null,
      title: "",
      prompt: "",
      mode: null,
      annotations: [],
      nextNumber: 1,
    });
  const beginEdit = (task: DemoTask) => {
    if (!task.objectId) return;
    const annotations = task.annotations?.map((value) => ({ ...value })) ?? [];
    setDraft({
      objectId: task.objectId,
      afterId: null,
      taskId: task.id,
      title: task.title,
      prompt: task.prompt ?? task.detail,
      mode: null,
      annotations,
      nextNumber: Math.max(0, ...annotations.map((value) => value.number)) + 1,
    });
  };
  const cancel = () => setDraft(null);
  const setTitle = (title: string) =>
    setDraft((value) => value && { ...value, title });
  const generateTitle = () =>
    setDraft(
      (value) => value && { ...value, title: suggestMockIterationTitle(value) },
    );
  const stopAnnotating = () =>
    setDraft((value) => value && { ...value, mode: null });
  const setPrompt = (prompt: string) =>
    setDraft((value) => value && { ...value, prompt });
  const toggleMode = (mode: PreviewMode) =>
    setDraft(
      (value) => value && { ...value, mode: value.mode === mode ? null : mode },
    );
  const add = ({ target, selection }: PreviewFeedback) => {
    if (selection.kind === "whole") return;
    setDraft((value) => {
      if (!value?.mode) return value;
      return {
        ...value,
        nextNumber: value.nextNumber + 1,
        annotations: [
          ...value.annotations,
          { number: value.nextNumber, target, selection, prompt: "" },
        ],
      };
    });
  };
  const edit = (number: number, prompt: string) =>
    setDraft(
      (value) =>
        value && {
          ...value,
          annotations: value.annotations.map((annotation) =>
            annotation.number === number
              ? { ...annotation, prompt }
              : annotation,
          ),
        },
    );
  // Keep numbers stable because the main prompt can refer to deleted numbers.
  const remove = (number: number) =>
    setDraft(
      (value) =>
        value && {
          ...value,
          annotations: value.annotations.filter(
            (annotation) => annotation.number !== number,
          ),
        },
    );
  return {
    draft,
    begin,
    beginEdit,
    cancel,
    setTitle,
    generateTitle,
    setPrompt,
    toggleMode,
    stopAnnotating,
    add,
    edit,
    remove,
  };
}

export type IterationDraftState = ReturnType<typeof useIterationDraft>;
