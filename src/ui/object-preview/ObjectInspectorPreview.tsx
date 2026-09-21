import { Icon } from "../Icon";
import { PreviewPrompt } from "./PreviewControls";
import { SelectablePreview } from "./SelectablePreview";
import {
  engineName,
  selectionName,
  targetName,
  type PreviewFeedback,
  type PreviewMode,
} from "./preview-target";

export function ObjectInspectorPreview({
  feedback,
  mode,
  setMode,
  change,
  inspect,
  notify,
}: {
  feedback: PreviewFeedback;
  mode: PreviewMode;
  setMode: (mode: PreviewMode) => void;
  change: (feedback: PreviewFeedback) => void;
  inspect: () => void;
  notify: (message: string) => void;
}) {
  const { target, selection } = feedback;
  const model =
    target.part?.objectType === "模型" ||
    (!target.part &&
      !!target.parentName &&
      target.object.objectType === "模型");
  const context = `${target.parentName ? `${target.parentName} / ` : ""}${targetName(target)} · ${target.version} · ${engineName(target.engine)} · ${selectionName(selection)}`;
  return (
    <section className="op-inspector-preview">
      <div className="op-inspector-preview-tools">
        <div className="op-segment" role="group" aria-label="画面操作">
          <button
            aria-pressed={mode === "point"}
            onClick={() => setMode("point")}
            title="点选"
            aria-label="预览点选"
          >
            <Icon name="target" />
          </button>
          <button
            aria-pressed={mode === "box"}
            onClick={() => setMode("box")}
            title="框选"
            aria-label="预览框选"
          >
            <Icon name="maximize" />
          </button>
          <button
            onClick={() => change({ target, selection: { kind: "whole" } })}
            disabled={selection.kind === "whole"}
            title="清除画面选区"
            aria-label="清除画面选区"
          >
            <Icon name="close" />
          </button>
        </div>
        <button onClick={inspect} title="放大查看" aria-label="放大查看与反馈">
          <Icon name="search" />
        </button>
      </div>
      <SelectablePreview
        key={target.key + target.engine}
        target={target}
        selected
        mode={mode}
        selection={selection}
        onSelect={change}
        className="op-object-card"
      />
      <div className="op-inspector-preview-source">
        {model ? (
          <div className="op-segment" role="group" aria-label="预览来源">
            {(["Blender", "Godot"] as const).map((engine) => (
              <button
                key={engine}
                aria-pressed={engine === target.engine}
                onClick={() =>
                  change({
                    target: { ...target, engine },
                    selection: { kind: "whole" },
                  })
                }
              >
                {engine}
              </button>
            ))}
          </div>
        ) : (
          <span>{engineName(target.engine)}</span>
        )}
        <span>
          {selection.kind === "whole"
            ? "整体"
            : selection.kind === "point"
              ? "已点选"
              : "已框选"}
        </span>
      </div>
      <PreviewPrompt
        key={target.key + target.engine}
        label="修改需求"
        placeholder={`描述希望如何修改${targetName(target)}…`}
        context={context}
        notify={notify}
      />
      <p className="op-muted">UI 演示 · 暂未连接引擎，提交仅展示反馈。</p>
    </section>
  );
}
