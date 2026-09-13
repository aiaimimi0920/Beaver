import { useEffect, useRef, useState } from "react";
import { Dialog, Field } from "./components";
import { Icon } from "./Icon";
import type { StudioNote } from "./studio-preview";

export function StudioDocuments({
  notes,
  change,
  request,
}: {
  notes: StudioNote[];
  change: (notes: StudioNote[]) => void;
  request: (goal: string, refs: string[]) => void;
}) {
  const [selected, setSelected] = useState("character");
  const [tabs, setTabs] = useState(["character", "world"]);
  const [editing, setEditing] = useState(false);
  const [side, setSide] = useState<"outline" | "ai" | "closed">("outline");
  const [search, setSearch] = useState("");
  const [prompt, setPrompt] = useState("");
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [conflict, setConflict] = useState(false);
  const [saveForAI, setSaveForAI] = useState(false);
  const article = useRef<HTMLElement>(null);
  const note = notes.find((item) => item.id === selected);
  const text = note?.draft ?? note?.text ?? "";
  const dirty = !!note && note.draft !== undefined && note.draft !== note.text;
  const update = (patch: Partial<StudioNote>) =>
    change(notes.map((n) => (n.id === selected ? { ...n, ...patch } : n)));
  const save = () => {
    if (note) update({ text, draft: undefined });
  };
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (!conflict && !creating && !saveForAI) save();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  });
  const open = (id: string) => {
    setSelected(id);
    setTabs((old) => (old.includes(id) ? old : [...old, id]));
    setPrompt("");
  };
  const submit = () => {
    if (!note || !prompt.trim()) return;
    request(prompt.trim(), [`资料/${note.folder}/${note.name}.md`]);
    setPrompt("");
  };
  const folders = [...new Set(notes.map((n) => n.folder))];
  return (
    <div className={`studio-docs ${side === "closed" ? "side-closed" : ""}`}>
      <aside className="studio-directory">
        <div className="studio-toolbar">
          <strong>文件</strong>
          <button
            aria-label="新建文档"
            onClick={() => {
              setName("");
              setCreating(true);
            }}
          >
            <Icon name="add" />
          </button>
        </div>
        <input
          aria-label="搜索文档"
          placeholder="搜索文档"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <div className="studio-scroll">
          {folders.map((folder) => {
            const found = notes.filter(
              (n) =>
                n.folder === folder && `${n.name} ${n.text}`.includes(search),
            );
            return (
              found.length > 0 && (
                <details open key={folder}>
                  <summary>{folder}</summary>
                  {found.map((n) => (
                    <button
                      className={`studio-entry ${n.id === selected ? "active" : ""}`}
                      key={n.id}
                      onClick={() => open(n.id)}
                    >
                      <Icon name="book" />
                      <span>{n.name}.md</span>
                      {n.draft !== undefined && n.draft !== n.text && (
                        <span aria-label="未保存">●</span>
                      )}
                    </button>
                  ))}
                </details>
              )
            );
          })}
          {!notes.some((n) => `${n.name} ${n.text}`.includes(search)) && (
            <p className="studio-empty">无匹配文档</p>
          )}
        </div>
      </aside>
      <section className="studio-note">
        <div
          className="studio-document-tabs"
          role="tablist"
          aria-label="打开的文档"
        >
          {tabs.map((id) => {
            const tab = notes.find((n) => n.id === id);
            return (
              tab && (
                <div key={id} className={selected === id ? "active" : ""}>
                  <button
                    role="tab"
                    aria-selected={selected === id}
                    onClick={() => open(id)}
                  >
                    {tab.name}.md
                  </button>
                  <button
                    aria-label={`关闭${tab.name}`}
                    onClick={() => {
                      const next = tabs.filter((t) => t !== id);
                      setTabs(next);
                      if (selected === id) setSelected(next.at(-1) ?? "");
                    }}
                  >
                    ×
                  </button>
                </div>
              )
            );
          })}
        </div>
        {note ? (
          <>
            <div className="studio-toolbar">
              <span className="muted">{note.folder}</span>
              <div className="studio-spacer" />
              <button onClick={() => setEditing(!editing)}>
                {editing ? "阅读" : "编辑"}
              </button>
              {(editing || dirty) && (
                <button disabled={!dirty} onClick={save}>
                  保存
                </button>
              )}
              <button onClick={() => setSide("ai")}>AI 修改</button>
              <button
                aria-label="文档大纲"
                onClick={() =>
                  setSide(side === "outline" ? "closed" : "outline")
                }
              >
                <Icon name="project" />
              </button>
              <button
                title="预览其他任务同时修改文档的情境"
                onClick={() => setConflict(true)}
              >
                版本冲突
              </button>
            </div>
            {editing ? (
              <textarea
                className="studio-source"
                aria-label="文档原文"
                spellCheck={false}
                value={text}
                onChange={(e) => update({ draft: e.target.value })}
              />
            ) : (
              <article ref={article} className="studio-note-body studio-scroll">
                {text.split("\n").map((line, index) => {
                  if (line.startsWith("# "))
                    return <h1 key={index}>{line.slice(2)}</h1>;
                  if (line.startsWith("## "))
                    return (
                      <h2 id={`note-heading-${index}`} key={index}>
                        {line.slice(3)}
                      </h2>
                    );
                  return (
                    <p key={index}>
                      {line.split(/(\[\[.*?\]\])/).map((part, i) => {
                        const linked = part.startsWith("[[")
                          ? notes.find((n) => n.name === part.slice(2, -2))
                          : undefined;
                        return linked ? (
                          <button
                            className="studio-link"
                            key={i}
                            onClick={() => open(linked.id)}
                          >
                            {linked.name}
                          </button>
                        ) : (
                          part
                        );
                      })}
                    </p>
                  );
                })}
              </article>
            )}
            <footer className="studio-status">
              <span>{dirty ? "未保存" : "草稿已保存"}</span>
              <span>Markdown · {text.replace(/\s/g, "").length} 字</span>
            </footer>
          </>
        ) : (
          <div className="studio-empty">
            <button onClick={() => setCreating(true)}>新建文档</button>
          </div>
        )}
      </section>
      {side !== "closed" && (
        <aside className="studio-inspector">
          <div className="studio-toolbar">
            <button
              className={side === "outline" ? "active" : ""}
              onClick={() => setSide("outline")}
            >
              大纲
            </button>
            <button
              className={side === "ai" ? "active" : ""}
              onClick={() => setSide("ai")}
            >
              AI
            </button>
            <button aria-label="关闭文档侧栏" onClick={() => setSide("closed")}>
              ×
            </button>
          </div>
          {side === "outline" ? (
            <div className="studio-scroll studio-outline">
              {text.split("\n").map(
                (line, index) =>
                  line.startsWith("## ") && (
                    <button
                      key={index}
                      onClick={() => {
                        setEditing(false);
                        requestAnimationFrame(() =>
                          article.current
                            ?.querySelector(`#note-heading-${index}`)
                            ?.scrollIntoView({ block: "start" }),
                        );
                      }}
                    >
                      {line.slice(3)}
                    </button>
                  ),
              )}
              <details open>
                <summary>反向链接</summary>
                {notes
                  .filter(
                    (n) =>
                      note &&
                      n.id !== note.id &&
                      n.text.includes(`[[${note.name}]]`),
                  )
                  .map((n) => (
                    <button key={n.id} onClick={() => open(n.id)}>
                      {n.name}
                    </button>
                  ))}
              </details>
            </div>
          ) : (
            <div className="studio-ai">
              <span className="studio-chip">{note?.name ?? "未选择文档"}</span>
              <div className="studio-spacer" />
              <textarea
                aria-label="文档 AI 修改要求"
                placeholder="希望怎么改？"
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
              />
              <button
                className="primary"
                disabled={!note || !prompt.trim()}
                onClick={() => (dirty ? setSaveForAI(true) : submit())}
              >
                提交修改
              </button>
            </div>
          )}
        </aside>
      )}
      {creating && (
        <Dialog title="新建文档" close={() => setCreating(false)}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!name.trim()) return;
              const id = crypto.randomUUID();
              change([
                ...notes,
                {
                  id,
                  name: name.trim(),
                  folder: "未分类",
                  text: `# ${name.trim()}\n\n`,
                },
              ]);
              open(id);
              setEditing(true);
              setCreating(false);
            }}
          >
            <Field label="名称">
              <input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </Field>
            <footer>
              <button type="button" onClick={() => setCreating(false)}>
                取消
              </button>
              <button className="primary" disabled={!name.trim()}>
                创建
              </button>
            </footer>
          </form>
        </Dialog>
      )}
      {saveForAI && (
        <Dialog title="文档尚未保存" close={() => setSaveForAI(false)}>
          <p>保存当前草稿后提交？</p>
          <footer>
            <button onClick={() => setSaveForAI(false)}>取消</button>
            <button
              className="primary"
              onClick={() => {
                save();
                setSaveForAI(false);
                submit();
              }}
            >
              保存并提交
            </button>
          </footer>
        </Dialog>
      )}
      {conflict && note && (
        <Dialog title="文档已被其他任务修改" close={() => setConflict(false)}>
          <div className="studio-comparison">
            <section>
              <h3>当前版本</h3>
              <pre>{note.text}\n\n保留现有设定，不改变人物动机。</pre>
            </section>
            <section>
              <h3>我的草稿</h3>
              <pre>{text}</pre>
            </section>
          </div>
          <p className="muted">预览情境 · 不覆盖任何项目文件</p>
          <footer>
            <button onClick={() => setConflict(false)}>稍后处理</button>
            <button
              onClick={() => {
                const id = crypto.randomUUID();
                change([
                  ...notes,
                  {
                    id,
                    name: `${note.name} · 我的草稿`,
                    folder: note.folder,
                    text,
                  },
                ]);
                open(id);
                setConflict(false);
              }}
            >
              保留为副本
            </button>
            <button
              className="primary"
              onClick={() => {
                request("整合文档修改，保留双方意图，不覆盖人物设定。", [
                  note.name,
                ]);
                setConflict(false);
              }}
            >
              让 AI 整合
            </button>
          </footer>
        </Dialog>
      )}
    </div>
  );
}
