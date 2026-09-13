import { useEffect, useState } from "react";
import type { Task } from "../shared/types";
import { assetUrl, call, type Run } from "./api";
import { PagedText } from "./PagedText";
import { PagedEditor } from "./PagedEditor";
import { ModelPreview } from "./ModelPreview";
import { z } from "zod";

type Resource = { path: string; origin: string; exists: boolean };
type Document = { path: string; text: string; revision: string };

export function TaskResources({
  task,
  run,
  feedback,
}: {
  task: Task;
  run: Run;
  feedback: (path: string) => void;
}) {
  const [items, setItems] = useState<Resource[]>([]);
  const [selected, setSelected] = useState("");
  const [revision, refresh] = useState(0);
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [editing, setEditing] = useState<Document>();
  const [busy, setBusy] = useState(false);
  const draftKey = `beaver.task-document.${task.id}.${selected}`;
  useEffect(() => {
    if (!editing) return;
    try {
      localStorage.setItem(draftKey, JSON.stringify(editing));
    } catch {
      setError("草稿缓存失败，请保存或复制后离开。");
    }
  }, [draftKey, editing]);
  useEffect(() => {
    let live = true;
    setError("");
    void call<Resource[]>("task.resources", { id: task.id })
      .then((value) => {
        if (!live) return;
        setItems(value);
        setSelected((old) =>
          value.some((item) => item.path === old)
            ? old
            : (value[0]?.path ?? ""),
        );
      })
      .catch((e: unknown) => {
        if (live) setError(String(e));
      });
    return () => {
      live = false;
    };
  }, [task.id, revision]);
  const item = items.find((entry) => entry.path === selected);
  const image = /\.(png|jpe?g|webp|gif|bmp|svg)$/i.test(selected);
  const audio = /\.(wav|ogg|mp3|flac|m4a)$/i.test(selected);
  const readable =
    /\.(md|txt|log|json|gd|tscn|tres|cfg|godot|csv|po|pot)$/i.test(selected);
  const url = assetUrl(`task-${task.id}`, selected, String(revision));
  useEffect(() => {
    let live = true;
    setText("");
    setError("");
    if (item?.exists && readable)
      void call<string>("task.resourceText", { id: task.id, path: selected })
        .then((value) => {
          if (live) setText(value);
        })
        .catch((e: unknown) => {
          if (live) setText(String(e));
        });
    return () => {
      live = false;
    };
  }, [task.id, selected, revision, item?.exists, readable]);
  return (
    <section className="task-resources">
      <header>
        <select
          aria-label="任务相关文件"
          value={selected}
          disabled={!!editing || busy}
          onChange={(e) => setSelected(e.target.value)}
        >
          {!items.length && <option value="">暂无参考或修改文件</option>}
          {items.map((entry) => (
            <option key={entry.path} value={entry.path}>
              {entry.origin} · {entry.path}
              {entry.exists ? "" : "（已删除 / 不存在）"}
            </option>
          ))}
        </select>
        <button
          disabled={!!editing || busy}
          onClick={() => refresh((old) => old + 1)}
        >
          刷新
        </button>
        <button
          disabled={!item?.exists || busy}
          onClick={() =>
            void run(async () => {
              const data = await call<{ base64: string }>(
                "task.resourceBytes",
                { id: task.id, path: selected },
              );
              const bytes = Uint8Array.from(atob(data.base64), (c) =>
                c.charCodeAt(0),
              );
              const href = URL.createObjectURL(
                new Blob([bytes], { type: "application/octet-stream" }),
              );
              const a = document.createElement("a");
              a.href = href;
              a.download = selected.split(/[\\/]/).at(-1) ?? "resource";
              a.click();
              setTimeout(() => URL.revokeObjectURL(href), 1000);
            })
          }
        >
          下载原始文件
        </button>
      </header>
      <div className="resource-preview">
        {editing ? (
          <div className="resource-editor">
            <small>编辑项目当前版本 · 不覆盖执行副本</small>
            <PagedEditor
              label="修正相关文档"
              value={editing.text}
              disabled={busy}
              change={(value) => setEditing({ ...editing, text: value })}
            />
            <footer>
              <button
                disabled={busy}
                onClick={() => {
                  localStorage.removeItem(draftKey);
                  setEditing(undefined);
                }}
              >
                取消
              </button>
              <button
                className="primary"
                disabled={busy}
                onClick={() => {
                  setBusy(true);
                  void run(async () => {
                    await call("document.save", {
                      id: task.projectId,
                      ...editing,
                    });
                    localStorage.removeItem(draftKey);
                    setEditing(undefined);
                    feedback(
                      `${selected}（已人工修正项目版本，请检查差异并保留修正）`,
                    );
                  }).finally(() => setBusy(false));
                }}
              >
                保存修正
              </button>
            </footer>
          </div>
        ) : error ? (
          <PagedText text={error} label="文件读取错误" />
        ) : !item ? (
          <PagedText text="暂无相关文件" label="任务资源" />
        ) : !item.exists ? (
          <PagedText text="此文件不在任务副本中。" label="文件状态" />
        ) : image ? (
          <img
            src={url}
            alt={selected}
            onError={() => setError("图片预览失败，请刷新或打开工作副本。")}
          />
        ) : audio ? (
          <audio
            controls
            src={url}
            onError={() => setError("音频预览失败，请刷新或打开工作副本。")}
          />
        ) : /\.(glb|gltf|obj)$/i.test(selected) ? (
          <ModelPreview url={url} fit />
        ) : (
          <PagedText
            key={selected}
            text={readable ? text : "此格式请在工作副本中用对应工具查看。"}
            label="任务文件内容"
          />
        )}
      </div>
      <footer>
        <small>任务副本 · {items.length} 个相关文件</small>
        <button
          disabled={!selected || !!editing || busy}
          onClick={() => feedback(selected)}
        >
          针对文件反馈
        </button>
        {selected.endsWith(".md") && (
          <button
            disabled={!!editing || busy}
            onClick={() => {
              setBusy(true);
              void run(async () => {
                const cached = localStorage.getItem(draftKey);
                if (cached) {
                  const parsed = z
                    .object({
                      path: z.literal(selected),
                      text: z.string(),
                      revision: z.string().regex(/^[a-f0-9]{64}$/),
                    })
                    .safeParse(JSON.parse(cached));
                  if (parsed.success) {
                    setEditing(parsed.data);
                    return;
                  }
                }
                setEditing(
                  await call<Document>("document.read", {
                    id: task.projectId,
                    path: selected,
                  }),
                );
              }).finally(() => setBusy(false));
            }}
          >
            编辑项目文档
          </button>
        )}
      </footer>
    </section>
  );
}
