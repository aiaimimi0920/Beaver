import { useEffect, useState } from "react";
import { call } from "../api";
import { PreviewFrameRegions } from "./PreviewFrameRegions";
import { type SceneTarget } from "./object-scene-preview";
import {
  parseSavedFrames,
  type SavedPreviewFrame,
} from "./saved-preview-frames";

export function SavedPreviewFrames({
  target,
  runId,
  snapshotId,
  refresh,
}: {
  target: SceneTarget;
  runId: string;
  snapshotId: string;
  refresh: number;
}) {
  const [frames, setFrames] = useState<SavedPreviewFrame[]>([]);
  const [selected, setSelected] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    void call("validation.preview.saved", {
      projectId: target.projectId,
      runId,
    })
      .then((raw) => {
        const saved = parseSavedFrames(raw, target, runId, snapshotId);
        if (!active) return;
        setFrames(saved);
        setSelected(saved.at(-1)?.id ?? "");
        setError("");
      })
      .catch((reason: unknown) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, [target, runId, snapshotId, refresh]);
  const saved = frames.find((item) => item.id === selected);
  return (
    <section aria-label="已保存交互帧">
      <h5>已保存交互帧（{frames.length} / 8）</h5>
      {error && <p role="alert">{error}</p>}
      {frames.length > 0 && (
        <select
          aria-label="已保存交互帧"
          value={selected}
          onChange={(event) => setSelected(event.target.value)}
        >
          {frames.map((item, index) => (
            <option key={item.id} value={item.id}>
              {index + 1} · {item.savedAt} · {item.frame.width} ×{" "}
              {item.frame.height}
            </option>
          ))}
        </select>
      )}
      {saved && (
        <>
          {saved.selection ? (
            <PreviewFrameRegions
              key={saved.id}
              frame={saved.frame}
              selection={saved.selection}
            />
          ) : (
            <img
              src={saved.frame.dataUrl}
              alt="已保存交互帧"
              style={{ maxWidth: "100%" }}
            />
          )}
          <p style={{ overflowWrap: "anywhere" }}>
            快照 {saved.snapshotId} · 帧 {saved.frame.sequence} · 视角{" "}
            {saved.frame.revision}
            <br />
            图像 SHA-256 {saved.frame.sha256}
          </p>
          <details>
            <summary>冻结来源与相机</summary>
            <pre style={{ whiteSpace: "pre-wrap" }}>
              {JSON.stringify(
                {
                  source: saved.source,
                  camera: saved.frame.camera,
                  width: saved.frame.width,
                  height: saved.frame.height,
                },
                null,
                2,
              )}
            </pre>
          </details>
        </>
      )}
      <p>
        保存后可关闭会话并回看，不启动
        场景引擎。每次保存新增不可变图像；每次渲染最多 8 帧、合计 16
        MiB。版本编号帧可附加到发布后任务；尝试输出编号帧可在候选反馈中用于本轮返工或后续中修。三维命中能力以原帧记录为准，图片不替代验收。
      </p>
    </section>
  );
}
