import { useEffect, useState, useSyncExternalStore } from "react";
import type { ObjectReworkImage } from "../../shared/object-rework-image";
import type { ObjectAttemptFile } from "./object-attempt-file";
import type { FrozenFeedbackPng } from "./final-relocation-source";
import { RegionContextThumbnail } from "../object-preview/RegionContextThumbnail";
export function FrozenFeedbackImage({
  viewer,
  image,
  onLoaded,
}: {
  viewer: ObjectAttemptFile;
  image: ObjectReworkImage;
  onLoaded?: (image: FrozenFeedbackPng | undefined) => void;
}) {
  const [decoded, setDecoded] = useState<{
    base64: string;
    width: number;
    height: number;
  }>();
  const state = useSyncExternalStore(
    viewer.subscribe,
    viewer.getSnapshot,
    viewer.getSnapshot,
  );
  useEffect(() => {
    void viewer.open("output", image.path, image.sha256);
    return viewer.cancel;
  }, [viewer, image.path, image.sha256]);
  const response = state.response;
  const content =
    response?.request.path === image.path &&
    response.request.sha256 === image.sha256
      ? response.content
      : undefined;
  const ready: FrozenFeedbackPng | undefined =
    response &&
    content?.kind === "image" &&
    content.mime === "image/png" &&
    !state.loading &&
    !state.error &&
    decoded?.base64 === content.base64 &&
    decoded.width === image.width &&
    decoded.height === image.height
      ? {
          kind: "png",
          request: response.request,
          image,
          width: decoded.width,
          height: decoded.height,
        }
      : undefined;
  const readinessKey = JSON.stringify(ready);
  useEffect(() => {
    onLoaded?.(ready);
    return () => onLoaded?.(undefined);
  }, [onLoaded, readinessKey]);
  return (
    <div>
      {state.loading && <p role="status">正在读取原始冻结图片…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      <button
        type="button"
        disabled={state.loading}
        onClick={() => void viewer.open("output", image.path, image.sha256)}
      >
        重新读取原始冻结图片
      </button>
      {content &&
        (content.kind === "image" && content.mime === "image/png" ? (
          <NumberedImage
            key={
              image.sha256 + image.width + ":" + image.height + content.base64
            }
            image={image}
            base64={content.base64}
            onLoaded={setDecoded}
          />
        ) : (
          <p role="alert">冻结文件不是可显示的 PNG，或已超过读取大小限制。</p>
        ))}
    </div>
  );
}

function NumberedImage({
  image,
  base64,
  onLoaded,
}: {
  image: ObjectReworkImage;
  base64: string;
  onLoaded: (
    image: { base64: string; width: number; height: number } | undefined,
  ) => void;
}) {
  const [url, setUrl] = useState("");
  const [status, setStatus] = useState<"loading" | "ready" | "error">(
    "loading",
  );
  useEffect(() => {
    onLoaded(
      status === "ready"
        ? { base64, width: image.width, height: image.height }
        : undefined,
    );
    return () => onLoaded(undefined);
  }, [onLoaded, status, base64, image.width, image.height]);
  useEffect(() => {
    setStatus("loading");
    let objectUrl = "";
    try {
      const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
      objectUrl = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
      setUrl(objectUrl);
    } catch {
      setStatus("error");
    }
    return () => {
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [base64]);
  return (
    <figure>
      {status === "loading" && <p role="status">正在校验图片尺寸…</p>}
      {status === "error" && (
        <p role="alert">
          原始图片解码失败或尺寸与已保存反馈不符，无法显示区域。
        </p>
      )}
      <div
        style={{ position: "relative", width: "fit-content", maxWidth: "100%" }}
      >
        {url && (
          <img
            src={url}
            alt={"原始反馈图片：" + image.path}
            style={{
              display: "block",
              maxWidth: "100%",
              maxHeight: "32rem",
              visibility: status === "ready" ? "visible" : "hidden",
            }}
            onLoad={(event) =>
              setStatus(
                event.currentTarget.naturalWidth === image.width &&
                  event.currentTarget.naturalHeight === image.height
                  ? "ready"
                  : "error",
              )
            }
            onError={() => setStatus("error")}
          />
        )}
        {status === "ready" && (
          <svg
            aria-label="已保存的编号区域"
            viewBox={`0 0 ${image.width} ${image.height}`}
            preserveAspectRatio="none"
            style={{
              position: "absolute",
              inset: 0,
              width: "100%",
              height: "100%",
              pointerEvents: "none",
            }}
          >
            {image.regions.map((region, index) => (
              <g key={index}>
                <title>
                  {index + 1}：{region.prompt || "使用整体修改意见"}
                </title>
                <rect
                  x={region.x * image.width}
                  y={region.y * image.height}
                  width={region.width * image.width}
                  height={region.height * image.height}
                  fill="#ffcc0033"
                  stroke="#ffcc00"
                  strokeWidth="2"
                />
                <text
                  x={region.x * image.width + 4}
                  y={region.y * image.height + 18}
                  fill="black"
                  stroke="white"
                  strokeWidth="1"
                  paintOrder="stroke"
                  fontSize="16"
                >
                  {index + 1}
                </text>
              </g>
            ))}
          </svg>
        )}
      </div>
      <figcaption>原始冻结成果；编号对应上方已保存意见。</figcaption>
      {status === "ready" &&
        image.regions.map((region, index) => (
          <RegionContextThumbnail
            key={index}
            source={url}
            width={image.width}
            height={image.height}
            region={region}
            number={index + 1}
          />
        ))}
    </figure>
  );
}
