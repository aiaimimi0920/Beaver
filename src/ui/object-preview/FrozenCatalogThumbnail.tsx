import { useEffect, useRef, useState } from "react";
import { assetUrl, call } from "../api";
import type { ObjectCatalogRecord } from "./use-object-catalog";
import {
  catalogSceneTarget,
  readCatalogThumbnail,
} from "./object-thumbnail-cache";

type Cached = Awaited<ReturnType<typeof readCatalogThumbnail>>;
export function FrozenCatalogThumbnail({
  object,
}: {
  object: ObjectCatalogRecord;
}) {
  const container = useRef<HTMLDivElement>(null);
  const [cached, setCached] = useState<Cached>(null);
  const [error, setError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const target = catalogSceneTarget(object);
  const identity = JSON.stringify(target);
  useEffect(() => {
    setCached(null);
    setError("");
    setLoaded(false);
    if (!target) return;
    const controller = new AbortController();
    let started = false;
    const read = () => {
      if (started || document.hidden || controller.signal.aborted) return;
      started = true;
      void readCatalogThumbnail(target, call, controller.signal)
        .then((value) => {
          if (!controller.signal.aborted) {
            setCached(value);
            setLoaded(true);
          }
        })
        .catch((reason: unknown) => {
          if (!controller.signal.aborted) {
            setError(String(reason));
            setLoaded(true);
          }
        });
    };
    let visible = false;
    const observer = new IntersectionObserver((entries) => {
      visible = entries.some((entry) => entry.isIntersecting);
      if (visible) read();
    });
    if (container.current) observer.observe(container.current);
    const wake = () => {
      if (visible) read();
    };
    document.addEventListener("visibilitychange", wake);
    return () => {
      controller.abort();
      observer.disconnect();
      document.removeEventListener("visibilitychange", wake);
    };
  }, [identity, object]);
  return (
    <div ref={container} className="op-object-thumbnail">
      {cached && !error ? (
        <>
          <img
            src={assetUrl(
              "validation",
              cached.result.run.id + "/" + cached.image.id,
            )}
            alt={object.name + " 冻结场景缓存"}
            loading="lazy"
            onError={() => setError("缓存图像不可用")}
            onLoad={(event) => {
              const size = cached.result.resolution;
              if (
                size &&
                (event.currentTarget.naturalWidth !== size.width ||
                  event.currentTarget.naturalHeight !== size.height)
              )
                setError("缓存图像尺寸不符");
            }}
          />
          <small
            title={
              "场景 " +
              cached.result.target.path +
              " · SHA-256 " +
              cached.image.sha256 +
              " · 快照 " +
              cached.result.run.snapshotId
            }
          >
            冻结版本{" "}
            {"versionId" in cached.result.target
              ? cached.result.target.versionId
              : ""}{" "}
            · 缓存
          </small>
        </>
      ) : (
        <span role="status">
          {error
            ? "冻结预览不可用"
            : loaded
              ? "尚无冻结场景缓存；在版本详情中渲染"
              : "等待读取冻结场景缓存"}
        </span>
      )}
      {error && <small title={error}>{error}</small>}
    </div>
  );
}
