import { useEffect, useRef, useState } from "react";
import type { PreviewPick, PreviewRectangle } from "../../shared/preview-pick";
import {
  normalizedRegion,
  type ImageRegion,
} from "../../shared/object-rework-image";
import type { PreviewSelection } from "../../shared/preview-selection";
import type { LiveFrame } from "./live-scene-preview";
import { RegionContextThumbnail } from "./RegionContextThumbnail";

export function PreviewFrameRegions({
  frame,
  selection,
  onChange,
  onPick,
  onReady,
  disabled = false,
}: {
  frame: LiveFrame;
  selection?: PreviewSelection;
  onChange?: (selection: PreviewSelection | undefined) => void;
  onPick?: (
    point: { x: number; y: number },
    rectangle?: PreviewRectangle,
  ) => Promise<PreviewPick>;
  disabled?: boolean;
  onReady?: (ready: boolean) => void;
}) {
  const [enabled, setEnabled] = useState(false);
  const [decoded, setDecoded] = useState(false);
  useEffect(() => {
    onReady?.(decoded);
    return () => onReady?.(false);
  }, [decoded, onReady]);
  const [error, setError] = useState("");
  const [pickMode, setPickMode] = useState(false);
  const [pickError, setPickError] = useState("");
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const start = useRef<{ x: number; y: number } | null>(null);
  const regions = selection?.regions ?? [];
  const writable = !!onChange && enabled && decoded && !disabled;
  function update(
    next: ImageRegion[],
    prompt = selection?.prompt ?? "",
    picks = selection?.picks ?? [],
  ) {
    onChange?.(
      next.length
        ? {
            kind: "image-regions",
            sequence: frame.sequence,
            sha256: frame.sha256,
            regions: next,
            prompt,
            coordinateSpace: "normalized-image",
            hitCapability: picks.length
              ? picks.some((p) => p.result.rectangle)
                ? "frozen-static-mesh"
                : "frozen-static-mesh-ray"
              : "unavailable",
            ...(picks.length ? { picks } : {}),
          }
        : undefined,
    );
  }
  function point(event: React.PointerEvent<SVGSVGElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    return {
      x: (event.clientX - bounds.left) / bounds.width,
      y: (event.clientY - bounds.top) / bounds.height,
    };
  }
  function addPick(
    region: ImageRegion,
    at: { x: number; y: number },
    rectangle?: PreviewRectangle,
  ) {
    setPickError("");
    void onPick?.(at, rectangle)
      .then((result) => {
        if (mounted.current)
          update([...regions, region], selection?.prompt ?? "", [
            ...(selection?.picks ?? []),
            { region: regions.length, result },
          ]);
      })
      .catch((reason: unknown) => {
        if (mounted.current) setPickError(String(reason));
      });
  }
  return (
    <section aria-label="冻结帧编号区域">
      {onChange && (
        <label>
          <input
            type="checkbox"
            checked={enabled}
            disabled={disabled}
            onChange={(event) => {
              start.current = null;
              setEnabled(event.target.checked);
            }}
          />
          启用图片框选
        </label>
      )}
      {onPick && (
        <label>
          <input
            type="checkbox"
            checked={pickMode}
            disabled={disabled}
            onChange={(event) => {
              start.current = null;
              setPickMode(event.target.checked);
            }}
          />
          使用三维点选（冻结静态网格）
        </label>
      )}
      <div
        style={{
          position: "relative",
          display: "inline-block",
          maxWidth: "100%",
        }}
      >
        <img
          src={frame.dataUrl}
          alt="冻结帧区域画面"
          draggable={false}
          style={{ display: "block", maxWidth: "100%", maxHeight: "32rem" }}
          onLoad={(event) => {
            const valid =
              event.currentTarget.naturalWidth === frame.width &&
              event.currentTarget.naturalHeight === frame.height;
            setDecoded(valid);
            setError(valid ? "" : "PREVIEW_FRAME_DIMENSIONS");
          }}
          onError={() => {
            setDecoded(false);
            setError("PREVIEW_FRAME_DECODE_FAILED");
          }}
        />
        {decoded && (
          <svg
            aria-label="冻结帧框选区域"
            viewBox="0 0 1000 1000"
            preserveAspectRatio="none"
            style={{
              position: "absolute",
              inset: 0,
              width: "100%",
              height: "100%",
              touchAction: "none",
              cursor: writable ? "crosshair" : "default",
            }}
            onPointerDown={(event) => {
              if (!writable || event.button !== 0 || regions.length >= 8)
                return;
              start.current = point(event);
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerUp={(event) => {
              const from = start.current;
              start.current = null;
              if (event.currentTarget.hasPointerCapture(event.pointerId))
                event.currentTarget.releasePointerCapture(event.pointerId);
              if (!from || !writable) return;
              if (pickMode && onPick) {
                const at = point(event);
                if (Math.hypot(at.x - from.x, at.y - from.y) > 0.01) return;
                const x = Math.max(0, Math.min(0.98, at.x - 0.01));
                const y = Math.max(0, Math.min(0.98, at.y - 0.01));
                addPick({ x, y, width: 0.02, height: 0.02, prompt: "" }, at);
                return;
              }
              const region = normalizedRegion(from, point(event));
              if (!region) return;
              if (onPick) {
                const { x, y, width, height } = region;
                addPick(
                  region,
                  { x: x + width / 2, y: y + height / 2 },
                  { x, y, width, height },
                );
              } else update([...regions, region]);
            }}
            onPointerCancel={() => {
              start.current = null;
            }}
            onLostPointerCapture={() => {
              start.current = null;
            }}
          >
            {regions.map((region, index) => (
              <g key={index}>
                <rect
                  x={region.x * 1000}
                  y={region.y * 1000}
                  width={region.width * 1000}
                  height={region.height * 1000}
                  fill="#ffcc0033"
                  stroke="#ffcc00"
                  strokeWidth="3"
                />
                <text
                  x={region.x * 1000 + 5}
                  y={region.y * 1000 + 30}
                  fontSize="28"
                  fill="#000"
                  stroke="#fff"
                  paintOrder="stroke"
                >
                  {index + 1}
                </text>
              </g>
            ))}
          </svg>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
      {pickError && <p role="alert">{pickError}</p>}
      <ol>
        {regions.map((region, index) => (
          <li key={index}>
            {selection?.picks
              ?.filter((p) => p.region === index)
              .map(({ result }) => (
                <p key={result.requestId} style={{ overflowWrap: "anywhere" }}>
                  {result.rectangle
                    ? `穿透框选静态网格：${result.nodePaths?.join("、") || "无命中"}${result.truncated ? "（集合已截断）" : ""}`
                    : result.hit
                      ? `三维命中：${result.hit.nodePath} · 位置 ${result.hit.position.join(", ")}`
                      : "未命中支持的静态网格"}
                  {" · "}跳过 {result.skipped}{" "}
                  个不支持或超限节点；历史拓扑需重新定位。
                </p>
              ))}
            {decoded && !error && (
              <RegionContextThumbnail
                source={frame.dataUrl}
                width={frame.width}
                height={frame.height}
                region={region}
                number={index + 1}
              />
            )}
            <label>
              区域 {index + 1} 意见（可选）
              <textarea
                value={region.prompt}
                readOnly={!onChange}
                disabled={disabled}
                onChange={(event) =>
                  update(
                    regions.map((item, i) =>
                      i === index
                        ? { ...item, prompt: event.target.value }
                        : item,
                    ),
                  )
                }
              />
            </label>
            {onChange && (
              <button
                type="button"
                disabled={disabled}
                onClick={() =>
                  update(
                    regions.filter((_, i) => i !== index),
                    selection?.prompt ?? "",
                    (selection?.picks ?? [])
                      .filter((p) => p.region !== index)
                      .map((p) => ({
                        ...p,
                        region: p.region > index ? p.region - 1 : p.region,
                      })),
                  )
                }
              >
                删除区域 {index + 1}
              </button>
            )}
          </li>
        ))}
      </ol>
      {regions.length > 0 && (
        <label>
          区域总体意见
          <textarea
            value={selection?.prompt ?? ""}
            readOnly={!onChange}
            disabled={disabled}
            onChange={(event) => update(regions, event.target.value)}
          />
        </label>
      )}
      <p>
        图片区域最多 8
        个，按原图归一化坐标保存。三维选择仅查询冻结的静态网格三角形，能力限制以引擎说明和跳过数量为准。
        框选按相机近远裁剪范围返回与区域相交的网格节点，包含被遮挡的网格，不表示可见像素命中；最多返回
        32
        个节点，超限会标记截断。没有引擎能力时仅保存图片区域。关闭开关保留已有区域，但不新增选择。恢复实时预览将清除未保存区域。
      </p>
    </section>
  );
}
