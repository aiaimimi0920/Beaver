import { useEffect, useState } from "react";
import type { Asset, Reference } from "../shared/types";
import { call, type Run } from "./api";
import { Dialog, Field } from "./components";
import { z } from "zod";

interface Document {
  path: string;
  text: string;
  revision: string | null;
}
const savedDrafts = z.record(
  z.string(),
  z.object({
    base: z.object({
      path: z.string(),
      text: z.string().max(512000),
      revision: z.string().nullable(),
    }),
    text: z.string().max(512000),
  }),
);
function loadDrafts(key: string) {
  try {
    return savedDrafts.parse(JSON.parse(localStorage.getItem(key) ?? "{}"));
  } catch {
    return {};
  }
}

export function DocumentsView({
  projectId,
  run,
  addReference,
}: {
  projectId: string;
  run: Run;
  addReference: (ref: Reference) => void;
}) {
  const [files, setFiles] = useState<string[]>([]);
  const [selected, setSelected] = useState("");
  const storageKey = `beaver.document-drafts.v1.${projectId}`;
  const [initial] = useState(() => loadDrafts(storageKey));
  const [documents, setDocuments] = useState<Record<string, Document>>(() =>
    Object.fromEntries(
      Object.entries(initial).map(([key, value]) => [key, value.base]),
    ),
  );
  const [drafts, setDrafts] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      Object.entries(initial).map(([key, value]) => [key, value.text]),
    ),
  );
  const [storageError, setStorageError] = useState(false);
  useEffect(() => {
    try {
      localStorage.setItem(
        storageKey,
        JSON.stringify(
          Object.fromEntries(
            Object.entries(documents)
              .filter(
                ([key, value]) =>
                  value.revision === null ||
                  (drafts[key] !== undefined && drafts[key] !== value.text),
              )
              .map(([key, base]) => [
                key,
                { base, text: drafts[key] ?? base.text },
              ]),
          ),
        ),
      );
      setStorageError(false);
    } catch {
      setStorageError(true);
    }
  }, [documents, drafts, storageKey]);
  const [query, setQuery] = useState("");
  const [newPath, setNewPath] = useState("docs/world/世界观.md");
  const [creating, setCreating] = useState(false);
  const [reload, setReload] = useState(false);
  const [busy, setBusy] = useState(false);
  const doc = documents[selected];
  const text = drafts[selected] ?? doc?.text ?? "";
  const dirty = !!doc && text !== doc.text;
  async function refresh() {
    const assets = await call<Asset[]>("assets", { id: projectId });
    setFiles(assets.filter((a) => a.path.endsWith(".md")).map((a) => a.path));
  }
  useEffect(() => {
    void run(refresh);
  }, [projectId]);
  async function open(file: string, force = false) {
    if (!documents[file] || force) {
      const value = await call<Document>("document.read", {
        id: projectId,
        path: file,
      });
      setDocuments((old) => ({ ...old, [file]: value }));
      if (force)
        setDrafts((old) => {
          const next = { ...old };
          delete next[file];
          return next;
        });
    }
    setSelected(file);
  }
  function save() {
    if (!doc || busy) return;
    const submitted = text;
    const file = selected;
    setBusy(true);
    void run(async () => {
      await call("document.save", {
        id: projectId,
        path: file,
        text: submitted,
        revision: doc.revision,
      });
      const value = await call<Document>("document.read", {
        id: projectId,
        path: file,
      });
      setDocuments((old) => ({ ...old, [file]: value }));
      setDrafts((old) => {
        const next = { ...old };
        if (next[file] === submitted) delete next[file];
        return next;
      });
      await refresh();
    }).finally(() => setBusy(false));
  }
  return (
    <div className="real-documents">
      {storageError && (
        <div role="alert">草稿缓存失败，请保存资料后再离开。</div>
      )}
      <aside>
        <header>
          <input
            aria-label="查找资料"
            placeholder="查找资料"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button onClick={() => setCreating(true)}>＋</button>
          <button aria-label="刷新资料" onClick={() => void run(refresh)}>
            ↻
          </button>
        </header>
        {[...new Set([...files, ...Object.keys(documents)])]
          .filter((file) => file.toLowerCase().includes(query.toLowerCase()))
          .sort()
          .map((file) => (
            <button
              className={selected === file ? "selected" : ""}
              key={file}
              onClick={() => void run(() => open(file))}
            >
              {file}
              {drafts[file] !== undefined &&
              drafts[file] !== documents[file]?.text
                ? " *"
                : ""}
            </button>
          ))}
      </aside>
      <section>
        {doc ? (
          <>
            <header>
              <strong>
                {selected}
                {dirty ? " *" : ""}
              </strong>
              <button
                disabled={busy}
                onClick={() =>
                  dirty ? setReload(true) : void run(() => open(selected, true))
                }
              >
                重新读取
              </button>
              <button
                disabled={busy || (!dirty && doc.revision !== null)}
                onClick={save}
              >
                保存
              </button>
              <button
                onClick={() =>
                  addReference({
                    path: selected,
                    note: "请先读取这份资料，依据我的目标修改或补充。",
                  })
                }
                disabled={doc.revision === null || dirty}
              >
                交给 AI
              </button>
            </header>
            <textarea
              className="document-source"
              aria-label="资料正文"
              spellCheck={false}
              value={text}
              onChange={(e) =>
                setDrafts((old) => ({ ...old, [selected]: e.target.value }))
              }
              onKeyDown={(e) => {
                if ((e.ctrlKey || e.metaKey) && e.key === "s") {
                  e.preventDefault();
                  save();
                }
              }}
            />
          </>
        ) : (
          <div className="muted">选择资料</div>
        )}
      </section>
      <aside className="document-outline">
        {text
          .split("\n")
          .filter((line) => /^#{1,6} /.test(line))
          .map((line, i) => (
            <div key={i}>{line.replace(/^#+ /, "")}</div>
          ))}
      </aside>
      {creating && (
        <Dialog title="新建资料" close={() => setCreating(false)}>
          <Field label="文件路径">
            <input
              value={newPath}
              onChange={(e) => setNewPath(e.target.value)}
            />
          </Field>
          <footer>
            <button
              disabled={!newPath.endsWith(".md")}
              onClick={() => {
                if (files.includes(newPath) || documents[newPath]) {
                  void run(() => open(newPath));
                } else {
                  setDocuments((old) => ({
                    ...old,
                    [newPath]: { path: newPath, text: "", revision: null },
                  }));
                  setSelected(newPath);
                }
                setCreating(false);
              }}
            >
              创建
            </button>
          </footer>
        </Dialog>
      )}
      {reload && (
        <Dialog title="放弃当前草稿？" close={() => setReload(false)}>
          <footer>
            <button onClick={() => setReload(false)}>保留草稿</button>
            <button
              className="danger"
              onClick={() =>
                void run(async () => {
                  await open(selected, true);
                  setReload(false);
                })
              }
            >
              重新读取
            </button>
          </footer>
        </Dialog>
      )}
    </div>
  );
}
