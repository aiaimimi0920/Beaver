import { useEffect, useState, useSyncExternalStore } from "react";
import type { ObjectPublicationFollowup } from "./object-publication-followup";
import {
  savedSchema,
  type SavedPreviewFrame,
} from "../object-preview/saved-preview-frames";
import { PreviewFrameRegions } from "../object-preview/PreviewFrameRegions";

export function parsePublicationFrames(
  raw: unknown,
  session: ObjectPublicationFollowup,
) {
  const frames = savedSchema.array().max(8).parse(raw);
  const op = session.publication;
  for (const frame of frames) {
    const target = frame.source.target;
    if (
      frame.projectId !== op.request.projectId ||
      frame.source.runId !== frame.runId ||
      !("versionId" in target) ||
      target.projectId !== op.request.projectId ||
      target.objectId !== op.request.target.objectId ||
      target.versionId !== op.versionId ||
      !frame.selection ||
      !frame.frame.frozen ||
      frame.selection.sha256 !== frame.frame.sha256 ||
      frame.selection.sequence !== frame.frame.sequence
    )
      throw new Error("PREVIEW_FEEDBACK_SOURCE_MISMATCH");
  }
  return frames;
}

export function PublicationFramePicker({
  session,
}: {
  session: ObjectPublicationFollowup;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const [frames, setFrames] = useState<SavedPreviewFrame[]>([]);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let active = true;
    void session
      .frames()
      .then((raw) => {
        const result = parsePublicationFrames(raw, session);
        if (active) {
          setFrames(result);
          setError("");
        }
      })
      .catch((reason: unknown) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, [session, revision]);
  const reference = state.draft.previewFrame;
  const selected = frames.find(
    (item) => item.id === reference?.frameId && item.runId === reference.runId,
  );
  const locked = state.busy || state.retry || state.recoveryBlocked;
  return (
    <fieldset disabled={locked}>
      <legend>附加此版本的编号预览帧（可选）</legend>
      <p>
        先在对象版本的场景预览中冻结、框选并保存，再刷新此处。显示相同版本最近 8
        个编号帧；不会启动引擎。已绑定的旧帧引用仍保留。
      </p>
      <button onClick={() => setRevision((value) => value + 1)}>
        刷新已保存编号帧
      </button>
      {error && <p role="alert">{error}</p>}
      <select
        aria-label="后续任务编号帧"
        value={reference ? JSON.stringify(reference) : ""}
        onChange={(event) => {
          const frame = frames.find(
            (item) =>
              JSON.stringify({ runId: item.runId, frameId: item.id }) ===
              event.target.value,
          );
          session.edit({
            previewFrame: frame
              ? { runId: frame.runId, frameId: frame.id }
              : undefined,
          });
        }}
      >
        <option value="">不附加图片</option>
        {reference && !selected && (
          <option value={JSON.stringify(reference)}>
            原帧尚未读取（保留引用）
          </option>
        )}
        {frames.map((frame) => (
          <option
            key={frame.id}
            value={JSON.stringify({ runId: frame.runId, frameId: frame.id })}
          >
            {frame.savedAt} · {frame.source.target.path} ·{" "}
            {frame.selection?.regions.length} 个区域
          </option>
        ))}
      </select>
      {selected && (
        <PreviewFrameRegions
          key={selected.id}
          frame={selected.frame}
          selection={selected.selection}
        />
      )}
      <p>
        附加原帧、相机、编号区域与保存时意见。二维区域不代表三维命中；后续任务仍须人工验收。
      </p>
      <p>
        模型图片传输上限为 1
        MiB；超过上限只返回大小限制与来源信息，请降低预览分辨率重新保存。
      </p>
    </fieldset>
  );
}
