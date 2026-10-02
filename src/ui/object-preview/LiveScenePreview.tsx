import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { call } from "../api";
import { PreviewFrameRegions } from "./PreviewFrameRegions";
import type { PreviewSelection } from "../../shared/preview-selection";
import {
  LiveScenePreview as Session,
  initialCamera,
  resolutions,
} from "./live-scene-preview";

export function LiveScenePreview({
  projectId,
  runId,
  snapshotId,
  engine = "Godot",
  onClose,
  onSaved,
}: {
  projectId: string;
  runId: string;
  snapshotId: string;
  engine?: "Godot" | "Blender";
  onClose: () => void;
  onSaved?: () => void;
}) {
  const [session] = useState(
    () => new Session(projectId, runId, snapshotId, call),
  );
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const camera = useRef(initialCamera());
  const drag = useRef<{ x: number; y: number; pan: boolean } | null>(null);
  const [decodeError, setDecodeError] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [selection, setSelection] = useState<PreviewSelection>();
  useEffect(() => {
    setSelection(undefined);
  }, [state.revision]);
  useEffect(() => {
    const refresh = () => {
      if (!document.hidden) void session.refresh();
    };
    refresh();
    const timer = setInterval(refresh, 500);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", refresh);
      session.close();
    };
  }, [session]);
  const frame = state.frame;
  const closing = state.closeStatus !== "idle";
  const clamp = (value: number, limit: number) =>
    Math.max(-limit, Math.min(limit, value));
  return (
    <section aria-label={engine + " 交互预览"}>
      <p>
        左键拖动旋转，Shift + 拖动平移，滚轮缩放。使用冻结快照的独立 {engine}
        会话；
        {engine === "Blender"
          ? "按相机操作渲染静态场景，不播放动画。"
          : "需要场景中已有 Camera3D。"}
        隐藏后停止取帧，15 秒无读取自动关闭，不影响制作任务。
      </p>
      <button
        type="button"
        disabled={
          saving || state.pickPending || state.closeStatus === "pending"
        }
        onClick={() => {
          drag.current = null;
          void session.requestClose().then((confirmed) => {
            if (confirmed) onClose();
          });
        }}
      >
        {state.closeStatus === "pending"
          ? "正在关闭交互预览…"
          : state.closeStatus === "failed"
            ? "重试关闭交互预览"
            : "关闭交互预览"}
      </button>
      {state.closeStatus === "pending" && (
        <p role="status">正在等待当前预览会话收尾，尚未确认关闭。</p>
      )}
      {state.closeError && (
        <p role="alert">
          关闭尚未确认，已保留原会话身份。可重试关闭；不会关闭其他会话。
          {state.closeError}
        </p>
      )}
      <button
        type="button"
        disabled={
          saving ||
          closing ||
          state.pickPending ||
          state.status !== "ready" ||
          !frame ||
          frame.revision !== state.revision
        }
        onClick={() => {
          setSaving(true);
          void session
            .capture(selection)
            .then(() => {
              setSaveError("");
              onSaved?.();
            })
            .catch((error: unknown) => setSaveError(String(error)))
            .finally(() => setSaving(false));
        }}
      >
        {saving ? "保存中" : saveError ? "重试保存交互帧" : "捕获并保存一帧"}
      </button>
      {saveError && <p role="alert">{saveError}</p>}
      <button
        type="button"
        disabled={
          state.capturePending ||
          closing ||
          state.pickPending ||
          state.status !== "ready" ||
          !frame ||
          frame.revision !== state.revision
        }
        onClick={() => {
          drag.current = null;
          session.setFrozen(!state.frozen);
        }}
      >
        {state.frozen ? "恢复实时预览" : "冻结预览画面"}
      </button>
      <button
        type="button"
        disabled={closing || state.status !== "ready" || state.frozen}
        onClick={() => {
          camera.current = initialCamera();
          session.setCamera(camera.current);
        }}
      >
        重置相机
      </button>
      <label>
        预览分辨率
        <select
          aria-label="预览分辨率"
          value={state.resolution ?? ""}
          disabled={closing || state.status !== "ready" || state.frozen}
          onChange={(event) =>
            session.setResolution(
              event.target.value as keyof typeof resolutions,
            )
          }
        >
          {!state.resolution && (
            <option value="">
              {frame ? `${frame.width} × ${frame.height}` : "等待实际帧"}
            </option>
          )}
          {Object.keys(resolutions).map((key) => (
            <option key={key} value={key}>
              {key}
            </option>
          ))}
        </select>
      </label>
      <p role="status">
        {state.status} ·{" "}
        {frame && frame.revision < state.revision
          ? "更新视角或分辨率中（保留上一帧）"
          : frame?.frozen
            ? "已冻结，可保存原帧"
            : "当前视角"}
      </p>
      {(state.error || decodeError) && (
        <p role="alert">{state.error || decodeError}</p>
      )}
      {frame?.frozen && frame.revision === state.revision ? (
        <PreviewFrameRegions
          key={frame.sha256 + ":" + frame.sequence}
          frame={frame}
          selection={selection}
          onChange={setSelection}
          onPick={
            frame.picking?.capability === "frozen-static-mesh-ray"
              ? (point, rectangle) => session.pick(point, rectangle)
              : undefined
          }
          disabled={
            closing || saving || state.capturePending || state.pickPending
          }
        />
      ) : (
        frame && (
          <img
            src={frame.dataUrl}
            alt={engine + " 实时冻结场景"}
            draggable={false}
            style={{ maxWidth: "100%", touchAction: "none", cursor: "grab" }}
            onLoad={(event) =>
              setDecodeError(
                event.currentTarget.naturalWidth === frame.width &&
                  event.currentTarget.naturalHeight === frame.height
                  ? ""
                  : "PREVIEW_FRAME_DIMENSIONS",
              )
            }
            onError={() => setDecodeError("PREVIEW_FRAME_DECODE_FAILED")}
            onPointerDown={(event) => {
              if (
                event.button !== 0 ||
                closing ||
                state.status !== "ready" ||
                state.frozen
              )
                return;
              event.currentTarget.setPointerCapture(event.pointerId);
              drag.current = {
                x: event.clientX,
                y: event.clientY,
                pan: event.shiftKey,
              };
            }}
            onPointerMove={(event) => {
              const previous = drag.current;
              if (!previous || closing || state.frozen) return;
              const bounds = event.currentTarget.getBoundingClientRect();
              if (!bounds.width || !bounds.height) return;
              const dx = (event.clientX - previous.x) / bounds.width;
              const dy = (event.clientY - previous.y) / bounds.height;
              const view = { ...camera.current };
              if (previous.pan) {
                view.panX = clamp(view.panX - dx, 10);
                view.panY = clamp(view.panY + dy, 10);
              } else {
                view.yaw = clamp(view.yaw - dx * 180, 360);
                view.pitch = clamp(view.pitch - dy * 180, 85);
              }
              drag.current = {
                ...previous,
                x: event.clientX,
                y: event.clientY,
              };
              camera.current = view;
              session.setCamera(view);
            }}
            onPointerUp={(event) => {
              drag.current = null;
              event.currentTarget.releasePointerCapture(event.pointerId);
            }}
            onPointerCancel={() => {
              drag.current = null;
            }}
            onLostPointerCapture={() => {
              drag.current = null;
            }}
            onWheel={(event) => {
              if (closing || state.status !== "ready" || state.frozen) return;
              camera.current = {
                ...camera.current,
                zoom: clamp(camera.current.zoom + event.deltaY * 0.001, 4),
              };
              session.setCamera(camera.current);
            }}
          />
        )
      )}
      {frame && (
        <p style={{ overflowWrap: "anywhere" }}>
          {frame.width} × {frame.height} · 帧 {frame.sequence} · 视角{" "}
          {frame.revision} · {engine} {frame.engine}
          <br />
          快照 {snapshotId} · 图像 SHA-256 {frame.sha256}
        </p>
      )}
      <p>
        {engine === "Blender"
          ? "冻结保留确认帧并锁定相机；点选和穿透框选使用原帧静态网格，跳过修改器、形态键和实例。对象身份使用 /objects/ 转义名称，历史拓扑需重新定位。"
          : "冻结暂停 SceneTree 并保留确认帧；自定义线程和忽略暂停的脚本不保证停止。点选使用渲染帧边界复制的静态网格，不查询之后的实时场景。"}
        交互画面不计入验收；版本编号存档可附加到发布后任务，尝试输出编号存档可在候选反馈中用于本轮返工或后续中修。
      </p>
    </section>
  );
}
