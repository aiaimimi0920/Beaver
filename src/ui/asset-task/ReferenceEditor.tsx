import { useRef, useState, type PointerEvent } from "react";
import type {
  AssetAnnotation,
  AssetReference,
  Point,
} from "../../shared/asset-task";
import { assetUrl } from "../api";
import { AnnotationOverlay } from "./AnnotationOverlay";
import { imagePoint } from "./preview-controls";

const tools = { box: "框选", arrow: "箭头", brush: "画笔" } as const;

export function ReferenceEditor({
  taskId,
  reference,
  annotations,
  change,
  locked = false,
  clear,
}: {
  taskId: string;
  reference: AssetReference;
  annotations: AssetAnnotation[];
  change: (marks: AssetAnnotation[]) => void;
  locked?: boolean;
  clear?: () => void;
}) {
  const [tool, setTool] = useState<AssetAnnotation["kind"]>("box");
  const [drawing, setDrawing] = useState<AssetAnnotation | null>(null);
  const [error, setError] = useState("");
  const drag = useRef<AssetAnnotation | null>(null);
  const pointer = useRef<number | null>(null);
  const { frame, pick } = reference;
  const count = annotations.reduce((sum, mark) => sum + mark.points.length, 0);
  const point = (event: PointerEvent<HTMLDivElement>): Point | null =>
    imagePoint(
      event.currentTarget.getBoundingClientRect(),
      frame.width,
      frame.height,
      event.clientX,
      event.clientY,
    );
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const p = point(event);
    const mark = drag.current;
    if (!p || !mark || locked || pointer.current !== event.pointerId) return;
    if (mark.kind === "brush") {
      if (mark.points.length + count >= 4096) return;
      const last = mark.points[mark.points.length - 1];
      if (
        last &&
        Math.hypot(
          (p[0] - last[0]) * frame.width,
          (p[1] - last[1]) * frame.height,
        ) < 2
      )
        return;
      mark.points.push(p);
    } else mark.points[1] = p;
    setDrawing({ ...mark, points: [...mark.points] });
  };
  const stop = () => {
    drag.current = null;
    pointer.current = null;
    setDrawing(null);
  };
  return (
    <section className="asset-reference" aria-label="固定参考画面">
      <header>
        <strong>固定参考画面</strong>
        {!locked && clear && <button onClick={clear}>移除参考</button>}
      </header>
      <p className="asset-caption">
        {frame.width} × {frame.height} · 场景版本 {frame.sceneRevision} ·{" "}
        {new Date(frame.capturedAt).toLocaleTimeString()}
      </p>
      {!locked && (
        <div className="asset-toolbar" role="group" aria-label="标注工具">
          {Object.entries(tools).map(([value, label]) => (
            <button
              key={value}
              aria-pressed={tool === value}
              onClick={() => setTool(value as AssetAnnotation["kind"])}
            >
              {label}
            </button>
          ))}
          <button
            disabled={!annotations.length}
            onClick={() => change(annotations.slice(0, -1))}
          >
            撤销标注
          </button>
        </div>
      )}
      <div
        className={`asset-reference-canvas${locked ? " is-locked" : ""}`}
        onPointerDown={(event) => {
          if (
            locked ||
            drag.current ||
            !event.isPrimary ||
            event.button !== 0 ||
            annotations.length >= 64 ||
            count > 4094
          )
            return;
          const p = point(event);
          if (!p) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          pointer.current = event.pointerId;
          drag.current = { kind: tool, points: [p, p] };
          setDrawing(drag.current);
        }}
        onPointerMove={move}
        onPointerCancel={stop}
        onLostPointerCapture={stop}
        onPointerUp={(event) => {
          if (pointer.current !== event.pointerId) return;
          move(event);
          if (drag.current && !locked) change([...annotations, drag.current]);
          stop();
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
        }}
      >
        <img
          src={assetUrl("asset-task", `${taskId}/references/${reference.id}`)}
          draggable={false}
          alt="提交反馈所依据的固定截图"
          onError={() => setError("参考画面已不可读，请重新定格。")}
          onLoad={() => setError("")}
        />
        <AnnotationOverlay
          width={frame.width}
          height={frame.height}
          pick={pick}
          annotations={drawing ? [...annotations, drawing] : annotations}
        />
      </div>
      {error && <p role="alert">{error}</p>}
      {pick && (
        <p className="asset-caption">
          几何命中：{pick.objectName} · 局部 (
          {pick.local.map((n) => n.toFixed(3)).join(", ")})。部位含义由 AI
          结合画面核对。
        </p>
      )}
      {!locked && (
        <p className="asset-caption">
          标注只作用于这张截图；实时观察继续更新。{annotations.length} / 64
          处标注。
        </p>
      )}
    </section>
  );
}
