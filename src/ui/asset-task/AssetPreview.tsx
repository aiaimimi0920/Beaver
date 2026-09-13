import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import type {
  AssetFrame,
  AssetReference,
  Point,
} from "../../shared/asset-task";
import { assetUrl } from "../api";
import {
  defaultView,
  imagePoint,
  moveView,
  zoomView,
} from "./preview-controls";
import type { useObserver } from "./use-observer";

export function AssetPreview({
  id,
  observer,
  finished,
  saved,
  locked,
  freeze,
  pick,
}: {
  id: string;
  observer: ReturnType<typeof useObserver>;
  finished: boolean;
  saved: AssetReference | null;
  locked: boolean;
  freeze: () => void;
  pick: (frame: AssetFrame, point: Point) => void;
}) {
  const [mode, setMode] = useState<"orbit" | "pan" | "pick">("orbit");
  const [imageError, setImageError] = useState("");
  const viewport = useRef<HTMLDivElement>(null);
  const drag = useRef<{
    pointer: number;
    x: number;
    y: number;
    pan: boolean;
    frame?: AssetFrame;
    point?: Point;
  } | null>(null);
  const { status, display, error, now, view, changeView } = observer;
  const frame = finished ? saved?.frame : display?.frame;
  const url = finished
    ? saved
      ? assetUrl("asset-task", `${id}/references/${saved.id}`)
      : undefined
    : display?.url;
  const connected = !finished && !!status?.connected;
  const current =
    connected &&
    !!frame &&
    frame.sessionId === status?.sessionId &&
    frame.generation === status?.generation;
  const age = frame ? Math.max(0, (now - frame.capturedAt) / 1000) : null;
  const label = finished
    ? saved
      ? "交付时保存的画面 · 已停止实时同步"
      : "任务已交付 · 没有保存的画面"
    : !connected
      ? "观察连接未就绪"
      : status?.busy
        ? `忙碌：${status.busy.operation}`
        : !current || age === null || age > 2
          ? "画面暂未更新"
          : "实时观察";
  const reset = () =>
    changeView({
      ...defaultView,
      width: view.current.width,
      height: view.current.height,
    });

  useEffect(() => {
    const node = viewport.current;
    if (!node || !current) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const delta =
        event.deltaY *
        (event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? node.clientHeight
            : 1);
      changeView(zoomView(view.current, delta));
    };
    node.addEventListener("wheel", wheel, { passive: false });
    return () => node.removeEventListener("wheel", wheel);
  }, [current, changeView, view]);

  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (
      !current ||
      !frame ||
      drag.current ||
      !event.isPrimary ||
      ![0, 1, 2].includes(event.button)
    )
      return;
    event.preventDefault();
    event.currentTarget.focus();
    const point = imagePoint(
      event.currentTarget.getBoundingClientRect(),
      frame.width,
      frame.height,
      event.clientX,
      event.clientY,
    );
    if (mode === "pick" && event.button === 0 && (locked || !point)) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = {
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      pan: mode === "pan" || event.shiftKey || event.button !== 0,
      ...(mode === "pick" && event.button === 0 && point
        ? { frame, point }
        : {}),
    };
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const previous = drag.current;
    if (
      !previous ||
      previous.pointer !== event.pointerId ||
      previous.frame ||
      !current
    )
      return;
    changeView(
      moveView(
        view.current,
        event.clientX - previous.x,
        event.clientY - previous.y,
        previous.pan,
      ),
    );
    previous.x = event.clientX;
    previous.y = event.clientY;
  };
  const key = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!current) return;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [-12, 0],
      ArrowRight: [12, 0],
      ArrowUp: [0, -12],
      ArrowDown: [0, 12],
    };
    const movement = moves[event.key];
    if (movement)
      changeView(
        moveView(view.current, movement[0], movement[1], event.shiftKey),
      );
    else if (event.key === "+" || event.key === "=")
      changeView(zoomView(view.current, -120));
    else if (event.key === "-") changeView(zoomView(view.current, 120));
    else if (event.key === "Home") reset();
    else return;
    event.preventDefault();
  };
  return (
    <section className="asset-live-preview" aria-label="资产观察视图">
      <header className="asset-preview-status">
        <strong data-connected={current}>{label}</strong>
        <span>
          {age === null ? "尚无画面" : `画面 ${age.toFixed(0)} 秒前更新`}
        </span>
      </header>
      <div className="asset-toolbar" role="group" aria-label="观察操作">
        <button
          disabled={!current}
          aria-pressed={mode === "orbit"}
          onClick={() => setMode("orbit")}
        >
          旋转
        </button>
        <button
          disabled={!current}
          aria-pressed={mode === "pan"}
          onClick={() => setMode("pan")}
        >
          平移
        </button>
        <button
          disabled={!current || locked}
          aria-pressed={mode === "pick"}
          onClick={() => setMode("pick")}
        >
          三维点选
        </button>
        <button
          disabled={!current}
          aria-label="拉近观察"
          onClick={() => changeView(zoomView(view.current, -200))}
        >
          ＋
        </button>
        <button
          disabled={!current}
          aria-label="拉远观察"
          onClick={() => changeView(zoomView(view.current, 200))}
        >
          −
        </button>
        <button disabled={!current} onClick={reset}>
          恢复视角
        </button>
        <button
          disabled={locked || (!current && !(finished && saved))}
          onClick={freeze}
        >
          定格并标注
        </button>
      </div>
      <div
        ref={viewport}
        className={`asset-viewport mode-${mode}`}
        tabIndex={0}
        aria-label="模型观察画面；方向键旋转，Shift 加方向键平移，加减号缩放，Home 恢复"
        onPointerDown={down}
        onPointerMove={move}
        onKeyDown={key}
        onContextMenu={(event) => event.preventDefault()}
        onPointerCancel={() => {
          drag.current = null;
        }}
        onLostPointerCapture={() => {
          drag.current = null;
        }}
        onPointerUp={(event) => {
          const selected = drag.current;
          if (selected?.pointer !== event.pointerId) return;
          drag.current = null;
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
          if (
            current &&
            !locked &&
            selected?.frame &&
            selected.point &&
            Math.hypot(event.clientX - selected.x, event.clientY - selected.y) <
              5
          )
            pick(selected.frame, selected.point);
        }}
      >
        {url ? (
          <img
            src={url}
            alt={
              finished ? "交付时保存的模型画面" : "当前任务 Blender 观察画面"
            }
            draggable={false}
            onLoad={() => setImageError("")}
            onError={() =>
              setImageError("画面无法读取，请等待下一帧或检查已保存资源。 ")
            }
          />
        ) : (
          <p className="asset-preview-empty">
            {finished
              ? "没有保存的交付画面。请从对话中的资源入口检查成果。"
              : "等待当前任务的 Blender 观察连接与第一帧。"}
          </p>
        )}
      </div>
      <p className="asset-caption">
        {finished
          ? "新要求将进入独立的后续任务。"
          : "拖动旋转 · Shift / 右键拖动平移 · 滚轮缩放。视角操作直接传给 Blender。"}
      </p>
      {frame && (
        <p className="asset-caption">
          场景 {frame.sceneRevision} · 视角 {frame.viewRevision} · {frame.width}{" "}
          × {frame.height} · {new Date(frame.capturedAt).toLocaleTimeString()}
        </p>
      )}
      {(error || status?.error || imageError) && (
        <p className="asset-notice" role="status">
          {imageError || error || status?.error}{" "}
          {display && !finished ? "保留最后可读画面。" : ""}
        </p>
      )}
      {mode === "pick" && !finished && (
        <p className="asset-caption">
          点击可见网格表面，定格并标记该帧的局部命中。场景已变化时需要重选。
        </p>
      )}
    </section>
  );
}
