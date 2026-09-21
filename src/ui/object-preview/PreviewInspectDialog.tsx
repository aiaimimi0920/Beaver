import { useState } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import { PreviewFeedbackBar } from "./PreviewFeedbackBar";
import { SelectablePreview } from "./SelectablePreview";
import {
  engineName,
  targetName,
  type PreviewFeedback,
  type PreviewMode,
} from "./preview-target";

export function PreviewInspectDialog({
  initial,
  close,
  change,
  notify,
}: {
  initial: PreviewFeedback;
  close: () => void;
  change: (feedback: PreviewFeedback) => void;
  notify: (message: string) => void;
}) {
  const [feedback, setFeedback] = useState(initial);
  const [mode, setMode] = useState<PreviewMode>("point");
  const update = (value: PreviewFeedback) => {
    setFeedback(value);
    change(value);
  };
  const { target } = feedback;
  const model =
    target.part?.objectType === "模型" ||
    (!target.part &&
      !!target.parentName &&
      target.object.objectType === "模型");
  return (
    <Dialog
      title={`观察 · ${targetName(target)}`}
      close={close}
      className="op-preview-inspect-dialog"
    >
      <div className="op-inspect-tools">
        <div className="op-segment" role="group" aria-label="预览来源">
          {(model ? (["Blender", "Godot"] as const) : [target.engine]).map(
            (engine) => (
              <button
                key={engine}
                aria-pressed={target.engine === engine}
                onClick={() =>
                  update({
                    target: { ...target, engine },
                    selection: { kind: "whole" },
                  })
                }
              >
                {engineName(engine)}
              </button>
            ),
          )}
        </div>
        <div className="op-segment" role="group" aria-label="画面操作">
          <button
            disabled
            title="绑定真实引擎画面后支持拖动旋转、平移与缩放"
            aria-label="旋转摄像机（未连接引擎）"
          >
            <Icon name="compass" />
          </button>
          <button
            aria-pressed={mode === "point"}
            onClick={() => setMode("point")}
            title="点选"
            aria-label="观察窗口点选"
          >
            <Icon name="target" />
          </button>
          <button
            aria-pressed={mode === "box"}
            onClick={() => setMode("box")}
            title="框选"
            aria-label="观察窗口框选"
          >
            <Icon name="maximize" />
          </button>
        </div>
      </div>
      <div className="op-inspect-canvas">
        <SelectablePreview
          key={target.key + target.engine}
          target={target}
          selected
          mode={mode}
          selection={feedback.selection}
          onSelect={update}
          className="op-object-card"
        />
      </div>
      <p className="op-inspect-source">
        {engineName(target.engine)} ·
        尚未绑定真实资源。当前为演示插画，切换来源仅预览界面；摄像机操作将在引擎接入后开放。
      </p>
      <PreviewFeedbackBar
        key={target.key + target.engine}
        feedback={feedback}
        close={close}
        inspect={() => {}}
        clearSelection={() => update({ target, selection: { kind: "whole" } })}
        notify={notify}
      />
    </Dialog>
  );
}
