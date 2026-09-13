import { useCallback, useEffect, useRef, useState } from "react";
import type { Asset, Reference } from "../shared/types";
import { assetUrl, call, size, type Run } from "./api";
import { Dialog, Empty, Field } from "./components";
import { ModelPreview } from "./ModelPreview";
export function AssetsView({
  projectId,
  addReferences,
  run,
}: {
  projectId: string;
  addReferences: (r: Reference[]) => void;
  run: Run;
}) {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [active, setActive] = useState<Asset>();
  const [text, setText] = useState<string>();
  const [textError, setTextError] = useState("");
  const [note, setNote] = useState("");
  const [region, setRegion] = useState<Reference["region"]>();
  const start = useRef<{ x: number; y: number } | undefined>(undefined);
  const [screens, setScreens] = useState<
    { id: string; name: string; image: string }[] | null
  >(null);
  const revision = useRef(0);
  const refresh = useCallback(async () => {
    const version = ++revision.current;
    const next = await call<Asset[]>("assets", { id: projectId });
    if (version !== revision.current) return;
    setAssets(next);
    setActive((a) => next.find((n) => n.path === a?.path));
    setSelected((paths) => paths.filter((p) => next.some((n) => n.path === p)));
  }, [projectId]);
  useEffect(() => {
    setSelected([]);
    setActive(undefined);
    void run(refresh);
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unsubscribe = window.beaver.subscribe(() => {
      clearTimeout(timer);
      timer = setTimeout(() => void run(refresh), 500);
    });
    return () => {
      revision.current++;
      clearTimeout(timer);
      unsubscribe();
    };
  }, [projectId]);
  useEffect(() => {
    setRegion(undefined);
    setText(undefined);
    setTextError("");
    let current = true;
    if (active && ["text", "scene"].includes(active.kind))
      void call<string>("asset.text", { id: projectId, path: active.path })
        .then((value) => {
          if (current) setText(value);
        })
        .catch((e) => {
          if (current) setTextError(String(e));
        });
    return () => {
      current = false;
    };
  }, [active?.path, active?.modifiedAt, projectId]);
  const filtered = assets.filter(
    (a) =>
      (filter === "all" || a.kind === filter) &&
      a.path.toLowerCase().includes(query.toLowerCase()),
  );
  function reference() {
    const chosen = selected.length ? selected : active ? [active.path] : [];
    addReferences(
      chosen.map((p) => ({
        path: p,
        note,
        region: p === active?.path ? region : undefined,
      })),
    );
  }
  return (
    <div className="asset-page">
      <div className="toolbar">
        <input
          aria-label="搜索素材"
          placeholder="搜索文件名或路径"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select
          aria-label="素材类型"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          {[
            ["all", "全部素材"],
            ["image", "图像"],
            ["audio", "音频"],
            ["model", "3D 模型"],
            ["scene", "场景"],
            ["text", "文本 / 翻译"],
            ["other", "其他"],
          ].map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </select>
        <button
          onClick={() =>
            void run(async () => {
              await call("asset.import", { id: projectId });
              await refresh();
            })
          }
        >
          导入素材
        </button>
        <button
          onClick={() =>
            void run(async () => setScreens(await call("screenshots")))
          }
        >
          截取窗口
        </button>
        <button onClick={() => void run(refresh)}>刷新</button>
      </div>
      <div className="asset-layout">
        <div className="asset-grid">
          {filtered.length === 0 ? (
            <Empty title="没有匹配的素材" />
          ) : (
            filtered.map((a) => (
              <article
                key={a.path}
                className={`asset-card ${active?.path === a.path ? "active" : ""}`}
              >
                <button
                  className="asset-open"
                  aria-label={`预览 ${a.path}`}
                  aria-pressed={active?.path === a.path}
                  onClick={() => setActive(a)}
                >
                  <div className="asset-thumbnail">
                    {a.kind === "image" ? (
                      <img
                        loading="lazy"
                        src={assetUrl(projectId, a.path, a.modifiedAt)}
                        alt={a.path}
                      />
                    ) : (
                      <span>
                        {a.kind === "audio"
                          ? "♫"
                          : a.kind === "model"
                            ? "⬡"
                            : a.kind === "scene"
                              ? "▧"
                              : "≡"}
                      </span>
                    )}
                  </div>
                  <strong title={a.path}>{a.path.split("/").pop()}</strong>
                  <small>
                    {a.kind} · {size(a.bytes)}
                  </small>
                </button>
                <input
                  aria-label={`选择 ${a.path}`}
                  type="checkbox"
                  checked={selected.includes(a.path)}
                  onChange={(e) =>
                    setSelected(
                      e.target.checked
                        ? [...selected, a.path]
                        : selected.filter((p) => p !== a.path),
                    )
                  }
                />
              </article>
            ))
          )}
        </div>
        <aside className="inspector">
          {active ? (
            <>
              <h3 className="filename">{active.path}</h3>
              {active.kind === "image" && (
                <div
                  className="image-frame"
                  title="拖动框选问题区域"
                  onPointerDown={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    start.current = {
                      x: (e.clientX - r.left) / r.width,
                      y: (e.clientY - r.top) / r.height,
                    };
                    e.currentTarget.setPointerCapture(e.pointerId);
                    setRegion(undefined);
                  }}
                  onPointerMove={(e) => {
                    if (!start.current) return;
                    const r = e.currentTarget.getBoundingClientRect();
                    const x = Math.min(
                      1,
                      Math.max(0, (e.clientX - r.left) / r.width),
                    );
                    const y = Math.min(
                      1,
                      Math.max(0, (e.clientY - r.top) / r.height),
                    );
                    setRegion({
                      x: Math.min(start.current.x, x),
                      y: Math.min(start.current.y, y),
                      w: Math.abs(x - start.current.x),
                      h: Math.abs(y - start.current.y),
                    });
                  }}
                  onPointerUp={() => {
                    start.current = undefined;
                  }}
                  onPointerCancel={() => {
                    start.current = undefined;
                  }}
                >
                  <img
                    draggable={false}
                    src={assetUrl(projectId, active.path, active.modifiedAt)}
                    alt="素材预览"
                  />
                  {region && (
                    <div
                      className="region"
                      style={{
                        left: `${region.x * 100}%`,
                        top: `${region.y * 100}%`,
                        width: `${region.w * 100}%`,
                        height: `${region.h * 100}%`,
                      }}
                    />
                  )}
                </div>
              )}
              {active.kind === "audio" && (
                <audio
                  controls
                  src={assetUrl(projectId, active.path, active.modifiedAt)}
                />
              )}
              {active.kind === "model" &&
                (active.path.toLowerCase().endsWith(".blend") ? (
                  <p className="muted">
                    .blend 暂不支持预览，可交给 Codex 转换为 GLB。
                  </p>
                ) : (
                  <ModelPreview
                    url={assetUrl(projectId, active.path, active.modifiedAt)}
                  />
                ))}
              {["text", "scene"].includes(active.kind) &&
                (textError ? (
                  <div className="error-box" role="alert">
                    {textError}
                  </div>
                ) : (
                  <pre className="text-preview">
                    {text === undefined ? "读取中…" : text || "空文件"}
                  </pre>
                ))}
              <button
                onClick={() =>
                  void run(() =>
                    call("asset.reveal", { id: projectId, path: active.path }),
                  )
                }
              >
                在文件夹中显示
              </button>
            </>
          ) : (
            <p className="muted">选择素材</p>
          )}
          <Field label={`反馈${region ? " · 已框选" : ""}`}>
            <textarea
              aria-label="素材反馈"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="修改要求、风格或场景对象…"
            />
          </Field>
          <button
            className="primary"
            disabled={!selected.length && !active}
            onClick={reference}
          >
            加入任务 · {selected.length || Number(!!active)}
          </button>
          <button
            onClick={() => {
              setSelected([]);
              setRegion(undefined);
            }}
          >
            清空选择
          </button>
        </aside>
      </div>
      {screens && (
        <Dialog title="选择需要反馈的游戏窗口" close={() => setScreens(null)}>
          <div className="capture-grid">
            {screens.map((s) => (
              <button
                key={s.id}
                onClick={() =>
                  void run(async () => {
                    const file = await call<string>("screenshot.capture", {
                      id: projectId,
                      source: s.id,
                    });
                    setScreens(null);
                    await refresh();
                    setSelected([file]);
                  })
                }
              >
                <img src={s.image} alt={s.name} />
                <span>{s.name}</span>
              </button>
            ))}
          </div>
        </Dialog>
      )}
    </div>
  );
}
