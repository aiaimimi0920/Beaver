import { useCallback, useEffect, useState } from "react";
import type { PreviewFrameReference } from "../../shared/object-publication";
import {
  savedSchema,
  type SavedPreviewFrame,
} from "../object-preview/saved-preview-frames";
import { PreviewFrameRegions } from "../object-preview/PreviewFrameRegions";
import type { ObjectPublication } from "./object-publication";

export function parseAttemptFrames(
  raw: unknown,
  projectId: string,
  runId: string,
  attemptId: string,
  reference?: PreviewFrameReference,
) {
  const frames = savedSchema.array().max(8).parse(raw);
  if (
    reference &&
    (frames.length !== 1 ||
      frames[0]?.id !== reference.frameId ||
      frames[0]?.runId !== reference.runId)
  )
    throw new Error("PREVIEW_FEEDBACK_REFERENCE_MISMATCH");
  for (const item of frames) {
    const target = item.source.target;
    if (
      item.projectId !== projectId ||
      item.source.runId !== item.runId ||
      !("attemptId" in target) ||
      target.projectId !== projectId ||
      target.runId !== runId ||
      target.attemptId !== attemptId ||
      target.checkpoint !== "output" ||
      !item.frame.frozen ||
      !item.selection ||
      item.selection.sequence !== item.frame.sequence ||
      item.selection.sha256 !== item.frame.sha256
    )
      throw new Error("PREVIEW_FEEDBACK_ATTEMPT_MISMATCH");
  }
  return frames;
}

export function AttemptFramePicker({
  session,
  attemptId,
  reference,
  onChange,
  onLoaded,
  label = "候选输出的编号存档帧",
  disabled = false,
  sourceRunId = session.review.target.runId,
}: {
  session: ObjectPublication;
  attemptId: string;
  reference?: PreviewFrameReference;
  onChange?: (reference: PreviewFrameReference | undefined) => void;
  onLoaded?: (frame: SavedPreviewFrame | undefined) => void;
  label?: string;
  disabled?: boolean;
  sourceRunId?: string;
}) {
  const [frames, setFrames] = useState<SavedPreviewFrame[]>([]);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const editable = !!onChange;
  const runId = reference?.runId;
  const frameId = reference?.frameId;
  const scope = JSON.stringify([
    session.review,
    attemptId,
    sourceRunId,
    runId,
    frameId,
    revision,
  ]);
  const [loadedScope, setLoadedScope] = useState("");
  const [readableScope, setReadableScope] = useState("");
  useEffect(() => {
    let active = true;
    setFrames([]);
    setError("");
    const bound = runId && frameId ? { runId, frameId } : undefined;
    const parse = (raw: unknown, ref?: PreviewFrameReference) =>
      parseAttemptFrames(
        raw,
        session.review.projectId,
        sourceRunId,
        attemptId,
        ref,
      );
    void (async () => {
      const recent = editable
        ? parse(await session.attemptFrames(attemptId))
        : [];
      const exact = bound
        ? parse(await session.attemptFrames(attemptId, bound), bound)
        : [];
      if (active) {
        setLoadedScope(scope);
        setFrames([
          ...exact,
          ...recent.filter(
            (item) =>
              !exact.some(
                (saved) => saved.id === item.id && saved.runId === item.runId,
              ),
          ),
        ]);
      }
    })().catch((reason: unknown) => {
      if (active) {
        setFrames([]);
        setError(String(reason));
      }
    });
    return () => {
      active = false;
    };
  }, [
    session,
    attemptId,
    sourceRunId,
    runId,
    frameId,
    editable,
    revision,
    scope,
  ]);
  const visible = loadedScope === scope ? frames : [];
  const selected = visible.find(
    (item) => item.id === frameId && item.runId === runId,
  );
  const ready = useCallback(
    (value: boolean) => setReadableScope(value ? scope : ""),
    [scope],
  );
  useEffect(() => {
    onLoaded?.(readableScope === scope ? selected : undefined);
    return () => onLoaded?.(undefined);
  }, [onLoaded, selected, readableScope, scope]);
  return (
    <fieldset disabled={disabled}>
      <legend>{label}</legend>
      {editable && (
        <p>
          在此尝试的输出场景预览中冻结、框选并保存，然后刷新。显示最近 8
          帧，已绑定的历史帧单独读取。
        </p>
      )}
      <button type="button" onClick={() => setRevision((value) => value + 1)}>
        刷新编号存档
      </button>
      {error && <p role="alert">{error}</p>}
      {editable && (
        <select
          aria-label={
            label === "候选输出的编号存档帧" ? "候选反馈编号帧" : label
          }
          value={reference ? JSON.stringify(reference) : ""}
          onChange={(event) => {
            const frame = visible.find(
              (item) =>
                JSON.stringify({ runId: item.runId, frameId: item.id }) ===
                event.target.value,
            );
            onChange?.(
              frame ? { runId: frame.runId, frameId: frame.id } : undefined,
            );
          }}
        >
          <option value="">不附加存档帧</option>
          {reference && !selected && (
            <option value={JSON.stringify(reference)}>
              原帧尚未读取（保留引用）
            </option>
          )}
          {visible.map((item) => (
            <option
              key={item.id}
              value={JSON.stringify({ runId: item.runId, frameId: item.id })}
            >
              {item.savedAt} · {item.source.target.path} ·{" "}
              {item.selection?.regions.length} 个区域
            </option>
          ))}
        </select>
      )}
      {reference && (
        <p>
          原始存档：{reference.runId} / {reference.frameId}
        </p>
      )}
      {selected && (
        <PreviewFrameRegions
          key={scope + selected.id}
          frame={selected.frame}
          selection={selected.selection}
          onReady={ready}
        />
      )}
      <p>
        保留原
        PNG、编号意见及相机来源。二维区域不代表三维命中；发布后须重新定位。模型图片传输上限为
        1 MiB，超限只返回来源及大小限制。
      </p>
    </fieldset>
  );
}
