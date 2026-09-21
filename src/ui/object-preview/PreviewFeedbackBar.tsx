import { useState } from "react";
import { Icon } from "../Icon";
import {
  engineName,
  selectionName,
  targetName,
  type PreviewFeedback,
} from "./preview-target";

export function PreviewFeedbackBar({
  feedback,
  close,
  inspect,
  clearSelection,
  notify,
}: {
  feedback: PreviewFeedback;
  close: () => void;
  inspect: () => void;
  clearSelection: () => void;
  notify: (message: string) => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [submitted, setSubmitted] = useState("");
  const { target, selection } = feedback;
  return (
    <form
      className="op-feedback-bar"
      aria-label="选中内容的修改需求"
      onSubmit={(event) => {
        event.preventDefault();
        if (!prompt.trim()) return;
        const receipt = `${targetName(target)} · ${target.version} · ${engineName(target.engine)} · ${selectionName(selection)} · ${prompt.trim()}`;
        setSubmitted(receipt);
        notify(`已演示提交：${receipt}；未创建真实任务。`);
        setPrompt("");
      }}
    >
      <div className="op-feedback-context">
        <strong>
          {target.parentName ? `${target.parentName} / ` : ""}
          {targetName(target)}
        </strong>
        <span>
          {target.version} · {engineName(target.engine)} ·{" "}
          {selectionName(selection)}
        </span>
        <button
          type="button"
          onClick={clearSelection}
          disabled={selection.kind === "whole"}
        >
          改为整体
        </button>
        <button
          type="button"
          onClick={inspect}
          title="放大查看与反馈"
          aria-label="放大查看与反馈"
        >
          <Icon name="search" />
        </button>
        <button
          type="button"
          onClick={close}
          title="关闭修改需求"
          aria-label="关闭修改需求"
        >
          <Icon name="close" />
        </button>
      </div>
      <div className="op-feedback-input">
        <input
          aria-label="修改需求"
          value={prompt}
          onChange={(event) => {
            setPrompt(event.target.value);
            setSubmitted("");
          }}
          placeholder="描述希望如何修改选中的对象或画面区域…"
        />
        <button type="submit" disabled={!prompt.trim()}>
          <Icon name="add" />
          提交需求
        </button>
      </div>
      <small role="status" title={submitted || undefined}>
        {submitted
          ? `已演示提交 · ${submitted}`
          : "UI 演示 · 画面未连接引擎；点选与框选记录二维位置，提交不执行真实任务。"}
      </small>
    </form>
  );
}
