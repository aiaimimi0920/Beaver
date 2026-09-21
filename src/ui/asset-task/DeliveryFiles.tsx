import { useEffect, useRef, useState } from "react";
import type { DeliveryCandidate } from "../../shared/asset-delivery";
import { call, type Run } from "../api";

export function DeliveryFiles({
  candidate,
  run,
}: {
  candidate: DeliveryCandidate;
  run: Run;
}) {
  const [preview, setPreview] = useState<{
    path: string;
    url?: string;
    text?: string;
  } | null>(null);
  const [exported, setExported] = useState("");
  const active = useRef(true);
  const reading = useRef(0);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      reading.current++;
    };
  }, []);
  useEffect(
    () => () => {
      if (preview?.url) URL.revokeObjectURL(preview.url);
    },
    [preview],
  );
  const read = (path: string, download: boolean) => {
    const sequence = ++reading.current;
    return run(async () => {
      const response = await call<{ base64: string }>(
        "assetTask.deliveryFile",
        { id: candidate.taskId, candidateId: candidate.id, path },
      );
      if (!active.current || sequence !== reading.current) return;
      const bytes = Uint8Array.from(atob(response.base64), (c) =>
        c.charCodeAt(0),
      );
      const extension = path.split(".").pop()?.toLowerCase() || "";
      const mime: Record<string, string> = {
        png: "image/png",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        webp: "image/webp",
      };
      const type = mime[extension];
      if (download) {
        const url = URL.createObjectURL(
          new Blob([bytes], { type: "application/octet-stream" }),
        );
        const link = document.createElement("a");
        link.href = url;
        link.download = path.split("/").pop() || "candidate";
        link.click();
        window.setTimeout(() => URL.revokeObjectURL(url), 1000);
      } else if (type) {
        setPreview({
          path,
          url: URL.createObjectURL(new Blob([bytes], { type })),
        });
      } else if (["txt", "md", "json", "csv", "log"].includes(extension)) {
        setPreview({
          path,
          text:
            new TextDecoder().decode(bytes.slice(0, 512 * 1024)) +
            (bytes.length > 512 * 1024 ? "\n[预览已截断，请下载完整文件]" : ""),
        });
      } else {
        setPreview({
          path,
          text: "此格式请下载或导出后查看；.blend 文件需使用 Blender 打开。",
        });
      }
    });
  };
  return (
    <section className="delivery-files">
      <p>以下内容来自提交时的文件副本。左侧观察窗口显示实时场景。</p>
      <button
        onClick={() =>
          void run(async () => {
            const result = await call<{ path: string }>(
              "assetTask.deliveryExport",
              { id: candidate.taskId, candidateId: candidate.id },
            );
            if (active.current) setExported(result.path);
          })
        }
      >
        导出全部候选文件
      </button>
      {exported && (
        <p role="status">
          已导出：<code>{exported}</code>
        </p>
      )}
      <ul>
        {Object.entries(candidate.files).map(([path, hash]) => (
          <li key={path}>
            <strong>{path}</strong>
            <small>SHA-256：{hash}</small>
            <div className="asset-toolbar">
              <button onClick={() => void read(path, false)}>查看</button>
              <button onClick={() => void read(path, true)}>下载</button>
            </div>
          </li>
        ))}
      </ul>
      {preview && (
        <figure>
          <figcaption>
            {preview.path} · 候选 {candidate.id}
          </figcaption>
          {preview.url ? (
            <img src={preview.url} alt={`已提交文件 ${preview.path}`} />
          ) : (
            <pre>{preview.text}</pre>
          )}
        </figure>
      )}
    </section>
  );
}
