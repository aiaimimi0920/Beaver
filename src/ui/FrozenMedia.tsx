import { useEffect, useState } from "react";
import type { FrozenMediaContent } from "../shared/frozen-media";

export function FrozenMedia({
  content,
  name,
}: {
  content: FrozenMediaContent;
  name: string;
}) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState(false);
  useEffect(() => {
    const bytes = Uint8Array.from(atob(content.base64), (char) =>
      char.charCodeAt(0),
    );
    const source = URL.createObjectURL(
      new Blob([bytes], { type: content.mime }),
    );
    setUrl(source);
    setError(false);
    return () => URL.revokeObjectURL(source);
  }, [content]);
  return (
    <>
      <p>内容摘要已校验。</p>
      {error ? (
        <p role="alert">媒体解码失败或此设备不支持该格式。</p>
      ) : (
        url &&
        (content.kind === "image" ? (
          <img
            src={url}
            alt={name}
            style={{
              maxWidth: "100%",
              maxHeight: "32rem",
              objectFit: "contain",
            }}
            onError={() => setError(true)}
          />
        ) : (
          <audio
            src={url}
            aria-label={name}
            controls
            preload="metadata"
            onError={() => setError(true)}
          />
        ))
      )}
    </>
  );
}
