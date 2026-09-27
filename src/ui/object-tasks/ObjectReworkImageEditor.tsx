import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  normalizedRegion,
  objectReworkImageSchema,
  type ImageRegion,
  type ObjectReworkImage,
} from "../../shared/object-rework-image";
import type { ObjectCandidateReport } from "../../shared/object-candidate-review";
import type { ObjectAttemptFile } from "./object-attempt-file";
import type { FeedbackImageDraft } from "./object-candidate-feedback-draft";
import { RegionContextThumbnail } from "../object-preview/RegionContextThumbnail";

type Change = (image: ObjectReworkImage | undefined, pending: boolean) => void;

export function ObjectReworkImageEditor({
  viewer,
  report,
  onChange,
  disabled,
  draft,
  onDraftChange,
}: {
  viewer: ObjectAttemptFile;
  report: ObjectCandidateReport;
  onChange: Change;
  disabled: boolean;
  draft?: FeedbackImageDraft;
  onDraftChange?: (draft: FeedbackImageDraft | undefined) => void;
}) {
  const [path, setPath] = useState(draft?.path ?? "");
  const [restored] = useState(draft);
  useEffect(() => {
    if (!restored) return;
    onChange(undefined, true);
    if (
      report.files.some(
        (file) => file.path === restored.path && file.after === restored.sha256,
      )
    )
      void viewer.open("output", restored.path, restored.sha256);
  }, [viewer, restored, report]);
  const state = useSyncExternalStore(
    viewer.subscribe,
    viewer.getSnapshot,
    viewer.getSnapshot,
  );
  const files = report.files.filter(
    (file) => file.after && /\.png$/i.test(file.path),
  );
  const hash = files.find((file) => file.path === path)?.after;
  const response = state.response;
  const content =
    response?.request.path === path &&
    response.request.sha256 === hash &&
    response.request.checkpoint === "output"
      ? response.content
      : null;
  return (
    <fieldset disabled={disabled}>
      <legend>冻结 PNG 区域（可选）</legend>
      <select
        aria-label="反馈图片"
        value={path}
        onChange={(event) => {
          const selected = event.target.value;
          setPath(selected);
          onDraftChange?.(
            selected
              ? {
                  path: selected,
                  sha256: files.find((file) => file.path === selected)!.after!,
                  regions: [],
                }
              : undefined,
          );
          onChange(undefined, !!selected);
          if (selected)
            void viewer.open(
              "output",
              selected,
              files.find((file) => file.path === selected)!.after!,
            );
          else viewer.cancel();
        }}
      >
        <option value="">仅文字反馈</option>
        {files.map((file) => (
          <option key={file.path} value={file.path}>
            {file.path}
          </option>
        ))}
      </select>
      {path && (
        <p>
          拖动框选区域，最多 8 个。切换图片会清除框选。仅支持 1 MiB 内、最多
          1600 万像素的 PNG。
        </p>
      )}
      {path && state.loading && <p role="status">正在读取冻结图片…</p>}
      {path && state.error && <p role="alert">{state.error}</p>}
      {path && (!hash || (draft?.path === path && draft.sha256 !== hash)) && (
        <p role="alert">
          草稿图片与此审阅不一致，请重新选择；未自动改用新图片。
        </p>
      )}
      {path &&
        content &&
        (content.kind === "image" && content.mime === "image/png" && hash ? (
          <RegionCanvas
            key={`${path}:${hash}`}
            path={path}
            sha256={hash}
            base64={content.base64}
            onChange={onChange}
            disabled={disabled}
            initialRegions={
              draft?.path === path && draft.sha256 === hash ? draft.regions : []
            }
            onDraftChange={onDraftChange}
          />
        ) : (
          <p role="alert">
            此文件无法用于 PNG 框选，请选择其他文件或仅文字反馈。
          </p>
        ))}
    </fieldset>
  );
}

function RegionCanvas({
  path,
  sha256,
  base64,
  onChange,
  disabled,
  initialRegions,
  onDraftChange,
}: {
  path: string;
  sha256: string;
  base64: string;
  onChange: Change;
  disabled: boolean;
  initialRegions: ImageRegion[];
  onDraftChange?: (draft: FeedbackImageDraft | undefined) => void;
}) {
  const [url, setUrl] = useState("");
  const [size, setSize] = useState<{ width: number; height: number }>();
  const [regions, setRegions] = useState<ImageRegion[]>(initialRegions);
  const [error, setError] = useState("");
  const start = useRef<{ x: number; y: number } | null>(null);
  useEffect(() => {
    const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
    const source = URL.createObjectURL(
      new Blob([bytes], { type: "image/png" }),
    );
    setUrl(source);
    return () => URL.revokeObjectURL(source);
  }, [base64]);
  function update(next: ImageRegion[]) {
    setRegions(next);
    onDraftChange?.({ path, sha256, regions: next });
    const result = objectReworkImageSchema.safeParse({
      path,
      sha256,
      ...size,
      regions: next,
    });
    onChange(result.success ? result.data : undefined, !result.success);
  }
  function point(event: React.PointerEvent<SVGSVGElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: (event.clientX - rect.left) / rect.width,
      y: (event.clientY - rect.top) / rect.height,
    };
  }
  return (
    <div>
      {error && <p role="alert">{error}</p>}
      <div
        style={{
          position: "relative",
          display: "inline-block",
          maxWidth: "100%",
        }}
      >
        {url && (
          <img
            src={url}
            alt={`框选 ${path}`}
            draggable={false}
            style={{ display: "block", maxWidth: "100%", maxHeight: "32rem" }}
            onLoad={(event) => {
              const { naturalWidth: width, naturalHeight: height } =
                event.currentTarget;
              if (
                width > 16384 ||
                height > 16384 ||
                width * height > 16_777_216
              ) {
                setError("图片尺寸超过框选上限。");
                setSize(undefined);
                return;
              }
              setSize({ width, height });
              const result = objectReworkImageSchema.safeParse({
                path,
                sha256,
                width,
                height,
                regions,
              });
              onChange(
                result.success ? result.data : undefined,
                !result.success,
              );
            }}
            onError={() => {
              setSize(undefined);
              setError("PNG 解码失败。");
              onChange(undefined, true);
            }}
          />
        )}
        {size && !error && (
          <svg
            aria-label="图片框选区域"
            viewBox="0 0 1000 1000"
            preserveAspectRatio="none"
            style={{
              position: "absolute",
              inset: 0,
              width: "100%",
              height: "100%",
              touchAction: "none",
              cursor: "crosshair",
            }}
            onPointerDown={(event) => {
              if (disabled || event.button !== 0 || regions.length >= 8) return;
              start.current = point(event);
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerCancel={() => {
              start.current = null;
            }}
            onPointerUp={(event) => {
              const from = start.current;
              start.current = null;
              if (!from || disabled) return;
              const region = normalizedRegion(from, point(event));
              if (region) update([...regions, region]);
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
      <ol>
        {regions.map((region, index) => (
          <li key={index}>
            {size && !error && (
              <RegionContextThumbnail
                source={url}
                width={size.width}
                height={size.height}
                region={region}
                number={index + 1}
              />
            )}
            <label>
              区域 {index + 1} 意见（可选）
              <textarea
                maxLength={1000}
                value={region.prompt}
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
            <button
              type="button"
              onClick={() => update(regions.filter((_, i) => i !== index))}
            >
              删除区域 {index + 1}
            </button>
          </li>
        ))}
      </ol>
      {!regions.length && <p>请在图片上框选至少一个区域，或切回仅文字反馈。</p>}
    </div>
  );
}
