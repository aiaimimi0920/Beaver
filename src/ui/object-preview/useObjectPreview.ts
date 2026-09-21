import { useState } from "react";
import type { DemoObject } from "./mock-objects";
import { objectPreviewTargets } from "./object-preview-targets";
import { mainPreview, type PreviewFeedback } from "./preview-target";

export function useObjectPreview(object: DemoObject, keepIterationTab = false) {
  const [versions, setVersions] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<PreviewFeedback | null>(null);
  const [tab, setTab] = useState("迭代栈");
  const view = (value: DemoObject): DemoObject => {
    const version = versions[value.id];
    return version ? { ...value, version } : value;
  };
  const viewedObject = view(object);
  const choices = objectPreviewTargets(viewedObject);
  const choice = choices.find(
    (value) => value.target.key === selected?.target.key,
  ) ??
    choices[0] ?? {
      value: "",
      label: "整个对象",
      target: mainPreview(viewedObject),
    };
  const feedback: PreviewFeedback =
    selected?.target.key === choice.target.key
      ? selected
      : { target: choice.target, selection: { kind: "whole" } };

  const selectFeedback = (value: PreviewFeedback) => {
    setSelected(value);
    if (!keepIterationTab && value.selection.kind !== "whole")
      setTab("预览与反馈");
  };
  const selectChild = (value: string) => {
    const target = choices.find((item) => item.value === value)?.target;
    if (target) selectFeedback({ target, selection: { kind: "whole" } });
  };
  const selectVersion = (version: string) => {
    const nextChoices = objectPreviewTargets({ ...object, version });
    const next = nextChoices.find((value) => value.value === choice.value);
    if (!next) return;
    // Viewing a snapshot never changes the current object or task route.
    setVersions((value) => ({ ...value, [object.id]: version }));
    selectFeedback({ target: next.target, selection: { kind: "whole" } });
  };
  const reset = () => {
    setSelected(null);
    setTab("迭代栈");
  };
  return {
    view,
    viewedObject,
    choices,
    child: choice.value,
    feedback,
    tab,
    setTab,
    selectFeedback,
    selectChild,
    selectVersion,
    reset,
  };
}

export type ObjectPreviewState = ReturnType<typeof useObjectPreview>;
