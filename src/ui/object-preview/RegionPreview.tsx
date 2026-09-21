import { useRef, useState, type PointerEvent } from "react";
import { Icon } from "../Icon";
import type { DemoObject } from "./mock-objects";
import { ObjectArt } from "./ObjectArt";
import { PreviewPrompt } from "./PreviewControls";

interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function RegionPreview({
  object,
  version,
  historical = false,
  notify,
  compact = false,
}: {
  object: DemoObject;
  version: string;
  historical?: boolean;
  notify: (message: string) => void;
  compact?: boolean;
}) {
  const [mode, setMode] = useState<"whole" | "region">("whole");
  const [region, setRegion] = useState<Region | null>(null);
  const start = useRef<{ x: number; y: number } | null>(null);
  const point = (event: PointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)),
      y: Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)),
    };
  };
  const finish = () => {
    start.current = null;
  };
  const context = `${object.id} · ${version}${region ? " · 已选区域" : " · 整体对象"}${historical ? " · 历史参考" : ""}`;
  return (
    <div className={`op-preview ${compact ? "op-preview-compact" : ""}`}>
      <div className="op-preview-tools">
        <div className="op-segment" aria-label="反馈范围">
          <button
            aria-pressed={mode === "whole"}
            onClick={() => {
              setMode("whole");
              setRegion(null);
            }}
          >
            <Icon name="features" />
            整体
          </button>
          <button
            aria-pressed={mode === "region"}
            onClick={() => setMode("region")}
          >
            <Icon name="maximize" />
            框选
          </button>
        </div>
        <span className="op-mono">
          {version} / {historical ? "历史快照" : "当前预览"}
        </span>
      </div>
      <div
        className={`op-preview-canvas ${mode === "region" ? "op-selecting" : ""}`}
      >
        <div
          className="op-art-surface"
          role="group"
          aria-label="对象演示画面"
          onPointerDown={(event) => {
            if (mode !== "region" || event.button !== 0) return;
            event.currentTarget.setPointerCapture(event.pointerId);
            const origin = point(event);
            start.current = origin;
            setRegion({ ...origin, width: 0, height: 0 });
          }}
          onPointerMove={(event) => {
            if (!start.current) return;
            const end = point(event),
              origin = start.current;
            setRegion({
              x: Math.min(origin.x, end.x),
              y: Math.min(origin.y, end.y),
              width: Math.abs(end.x - origin.x),
              height: Math.abs(end.y - origin.y),
            });
          }}
          onPointerUp={() => {
            finish();
            setRegion((value) =>
              value && value.width > 0.01 && value.height > 0.01 ? value : null,
            );
          }}
          onPointerCancel={finish}
          onLostPointerCapture={finish}
        >
          <ObjectArt kind={object.kind} historical={historical} />
          {region && (
            <div
              className="op-selection"
              style={{
                left: `${region.x * 100}%`,
                top: `${region.y * 100}%`,
                width: `${region.width * 100}%`,
                height: `${region.height * 100}%`,
              }}
            >
              <span>区域 01</span>
            </div>
          )}
        </div>
        <span className="op-visual-label">演示插画 · 非实际模型渲染</span>
        {mode === "region" && !region && (
          <span className="op-canvas-hint">拖动框选需要修改的区域</span>
        )}
      </div>
      <div className="op-preview-caption">
        <span>
          {object.name} · {version}
        </span>
        {mode === "region" ? (
          <button
            onClick={() =>
              region
                ? setRegion(null)
                : setRegion({ x: 0.38, y: 0.1, width: 0.23, height: 0.3 })
            }
          >
            {region ? "清除框选" : "选择示例区域"}
          </button>
        ) : (
          <span>测试画面</span>
        )}
      </div>
      <PreviewPrompt
        label="画面反馈"
        placeholder={
          region
            ? "描述选中区域需要怎样调整…"
            : "描述希望如何调整这个对象，也可以先框选区域…"
        }
        context={context}
        notify={notify}
      >
        {region && (
          <div className="op-reference-chip">
            <Icon name="maximize" />
            区域 01 · {Math.round(region.width * 100)}% ×{" "}
            {Math.round(region.height * 100)}%<span>已绑定 {version} 画面</span>
          </div>
        )}
      </PreviewPrompt>
    </div>
  );
}
