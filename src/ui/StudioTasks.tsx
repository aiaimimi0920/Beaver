import { useState } from "react";
import { Dialog, Field } from "./components";
import {
  nextPreviewStatus,
  pausePreviewTask,
  resumePreviewTask,
  previewStatus,
  type StudioTask,
} from "./studio-preview";

export function StudioTasks({
  tasks,
  selected,
  select,
  update,
  create,
  existing,
}: {
  tasks: StudioTask[];
  selected: string;
  select: (id: string) => void;
  update: (task: StudioTask) => void;
  create: (goal: string, refs: string[]) => void;
  existing?: () => void;
}) {
  const [filter, setFilter] = useState("全部任务");
  const [tab, setTab] = useState("结果");
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [newTask, setNewTask] = useState(false);
  const [goal, setGoal] = useState("");
  const [limits, setLimits] = useState("");
  const [rollback, setRollback] = useState(false);
  const [keep, setKeep] = useState<string[]>([]);
  const [method, setMethod] = useState("standard");
  const [decision, setDecision] = useState("");
  const current = tasks.find((t) => t.id === selected) ?? tasks[0];
  if (!current)
    return <button onClick={() => create("开始制作游戏", [])}>新任务</button>;
  const draft = drafts[current.id] ?? "";
  const amend = (message: string, resume: boolean) => {
    if (!message.trim()) return;
    update({
      ...current,
      messages: [...current.messages, message.trim()],
      status: resume ? "正在整合" : current.status,
    });
    setDrafts((old) => ({ ...old, [current.id]: "" }));
  };
  const stateClass =
    current.status === "需要你决定"
      ? "attention"
      : current.status === "已认可"
        ? "positive"
        : "";
  return (
    <div className="studio-tasks">
      <aside className="studio-directory">
        <div className="studio-toolbar">
          <strong>任务</strong>
          <button
            onClick={() => {
              setGoal("");
              setLimits("");
              setNewTask(true);
            }}
          >
            新任务
          </button>
        </div>
        <select
          aria-label="任务筛选"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          {["全部任务", ...previewStatus].map((s) => (
            <option key={s}>{s}</option>
          ))}
        </select>
        <div className="studio-scroll">
          {tasks
            .filter((t) => filter === "全部任务" || t.status === filter)
            .map((t) => (
              <button
                key={t.id}
                className={`studio-task-entry ${current.id === t.id ? "active" : ""}`}
                onClick={() => {
                  select(t.id);
                  setDecision("");
                }}
              >
                <strong>{t.goal}</strong>
                <small className={t.status === "需要你决定" ? "attention" : ""}>
                  {t.status}
                </small>
              </button>
            ))}
          {!tasks.some((t) => filter === "全部任务" || t.status === filter) && (
            <p className="studio-empty">暂无任务</p>
          )}
        </div>
        {existing && (
          <button className="studio-existing" onClick={existing}>
            已有任务 ↗
          </button>
        )}
      </aside>
      <section className="studio-task-detail">
        <div className="studio-toolbar">
          <h1>{current.goal}</h1>
          <span className={stateClass}>{current.status}</span>
          {["执行中", "正在整合"].includes(current.status) && (
            <button onClick={() => update(pausePreviewTask(current))}>
              中止
            </button>
          )}
          {current.status === "已暂停" && (
            <button onClick={() => update(resumePreviewTask(current))}>
              继续
            </button>
          )}
        </div>
        <div
          className="studio-toolbar studio-tabs"
          role="tablist"
          aria-label="任务内容"
        >
          {["结果", "对话", "变更与回退"].map((value) => (
            <button
              role="tab"
              aria-selected={tab === value}
              className={tab === value ? "active" : ""}
              key={value}
              onClick={() => setTab(value)}
            >
              {value}
            </button>
          ))}
          <div className="studio-spacer" />
          {["执行中", "正在整合"].includes(current.status) && (
            <button
              className="studio-link"
              title="仅推进界面示例，不执行或合入文件"
              onClick={() =>
                update({
                  ...current,
                  status: nextPreviewStatus(current.status),
                })
              }
            >
              预览下一步
            </button>
          )}
        </div>
        <div className="studio-scroll studio-task-body">
          {tab === "对话" ? (
            <>
              <div className="studio-message">
                <small>你</small>
                <p>{current.goal}</p>
              </div>
              {current.refs.length > 0 && (
                <div className="studio-chips">
                  {current.refs.map((ref, i) => (
                    <span className="studio-chip" key={i}>
                      {ref}
                    </span>
                  ))}
                </div>
              )}
              {current.messages.map((message, index) => (
                <div className="studio-message" key={index}>
                  <small>你 · 补充要求</small>
                  <p>{message}</p>
                </div>
              ))}
              <p className="muted">{current.status}</p>
            </>
          ) : tab === "变更与回退" ? (
            <>
              {[
                "scripts/save.gd",
                "scenes/save_menu.tscn",
                "docs/存档与验收.md",
              ].map((file) => (
                <div className="studio-file-change" key={file}>
                  <code>{file}</code>
                  <span>修改</span>
                </div>
              ))}
              <p className="muted">示例变更 · 当前项目未修改</p>
              <button
                className="danger"
                disabled={!["待验收", "已认可"].includes(current.status)}
                onClick={() => {
                  setKeep([]);
                  setMethod("standard");
                  setRollback(true);
                }}
              >
                回退此任务
              </button>
            </>
          ) : current.status === "需要你决定" ? (
            <div className="studio-conflict">
              <h2>
                {current.conflict === "binary"
                  ? "阿澄有两个待采用的版本"
                  : "存档格式需要统一"}
              </h2>
              <div className="studio-comparison">
                <section>
                  <span className="studio-chip">当前游戏</span>
                  <h3>
                    {current.conflict === "binary"
                      ? "保留防雨外套"
                      : "自动存档已采用新格式"}
                  </h3>
                  <p>
                    {current.conflict === "binary"
                      ? "暗色外套，克制的表情。"
                      : "对话进度和饮品选择分别保存。"}
                  </p>
                </section>
                <section>
                  <span className="studio-chip">本任务结果</span>
                  <h3>
                    {current.conflict === "binary"
                      ? "调整外套与表情"
                      : "读取界面使用旧格式"}
                  </h3>
                  <p>
                    {current.conflict === "binary"
                      ? "浅色外套，更明显的笑容。"
                      : "继续游戏只读取一个进度字段。"}
                  </p>
                </section>
              </div>
              <div className="studio-choice-list">
                {(current.conflict === "binary"
                  ? ["保留当前外套，采用新表情", "保留两个版本，稍后选择"]
                  : ["兼容现有存档，适配读取界面", "采用新格式，保留旧存档备份"]
                ).map((choice) => (
                  <label key={choice}>
                    <input
                      type="radio"
                      name="decision"
                      value={choice}
                      checked={decision === choice}
                      onChange={() => setDecision(choice)}
                    />
                    {choice}
                  </label>
                ))}
              </div>
              <Field label="补充要求">
                <textarea
                  value={draft}
                  onChange={(e) =>
                    setDrafts((old) => ({
                      ...old,
                      [current.id]: e.target.value,
                    }))
                  }
                  placeholder="也可以直接描述你希望保留的效果…"
                />
              </Field>
              <div className="studio-actions">
                <button onClick={() => update(pausePreviewTask(current))}>
                  稍后处理
                </button>
                <button
                  className="primary"
                  disabled={!decision && !draft.trim()}
                  onClick={() => {
                    amend([decision, draft].filter(Boolean).join("\n"), true);
                    setDecision("");
                  }}
                >
                  按此要求整合
                </button>
              </div>
            </div>
          ) : ["待验收", "已认可"].includes(current.status) ? (
            <>
              <div className="studio-result-placeholder">
                <span>游戏效果</span>
                <strong>{current.goal}</strong>
                <small>交互预览 · 无实际生成结果</small>
              </div>
              <div className="studio-actions">
                <button
                  className="primary"
                  disabled={current.status === "已认可"}
                  onClick={() => update({ ...current, status: "已认可" })}
                >
                  {current.status === "已认可" ? "已认可" : "认可结果"}
                </button>
                <button onClick={() => setTab("对话")}>继续调整</button>
                <button onClick={() => setTab("变更与回退")}>查看变更</button>
              </div>
            </>
          ) : (
            <div className="studio-progress">
              <h2>{current.status}</h2>
              <ol>
                {["任务成果", "与当前项目整合", "组合验证", "交付结果"].map(
                  (label, i) => (
                    <li
                      className={
                        current.status === "正在整合" && i === 1 ? "active" : ""
                      }
                      key={label}
                    >
                      {label}
                    </li>
                  ),
                )}
              </ol>
              {current.status === "已回退" && (
                <p>
                  回退预览完成
                  {current.kept?.length
                    ? `，保留 ${current.kept.length} 个文件`
                    : ""}
                  。
                </p>
              )}
            </div>
          )}
        </div>
        {current.status !== "需要你决定" && (
          <form
            className="studio-composer"
            onSubmit={(e) => {
              e.preventDefault();
              amend(draft, current.status !== "已暂停");
            }}
          >
            <textarea
              aria-label="任务补充要求"
              value={draft}
              onChange={(e) =>
                setDrafts((old) => ({ ...old, [current.id]: e.target.value }))
              }
              placeholder="指出哪里不对，或补充你想要的效果…"
            />
            <button className="primary" disabled={!draft.trim()}>
              {current.status === "已暂停" ? "保存补充" : "提交修改"}
            </button>
          </form>
        )}
      </section>
      {newTask && (
        <Dialog title="新建创作任务" close={() => setNewTask(false)}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!goal.trim()) return;
              create(
                [goal.trim(), limits.trim() ? `停止条件：${limits.trim()}` : ""]
                  .filter(Boolean)
                  .join("\n"),
                [],
              );
              setFilter("全部任务");
              setTab("对话");
              setNewTask(false);
            }}
          >
            <Field label="你希望完成什么？">
              <textarea
                value={goal}
                onChange={(e) => setGoal(e.target.value)}
                placeholder="描述你想要的游戏效果…"
              />
            </Field>
            <details>
              <summary>执行限制</summary>
              <Field label="在什么情况下停下">
                <input
                  value={limits}
                  onChange={(e) => setLimits(e.target.value)}
                />
              </Field>
            </details>
            <footer>
              <button type="button" onClick={() => setNewTask(false)}>
                取消
              </button>
              <button className="primary" disabled={!goal.trim()}>
                发起任务
              </button>
            </footer>
          </form>
        </Dialog>
      )}
      {rollback && (
        <Dialog title="回退此任务" close={() => setRollback(false)}>
          <div className="studio-choice-list">
            <label>
              <input
                type="radio"
                name="rollback"
                checked={method === "standard"}
                onChange={() => setMethod("standard")}
              />
              标准回退
            </label>
            <label>
              <input
                type="radio"
                name="rollback"
                checked={method === "ai"}
                onChange={() => setMethod("ai")}
              />
              对话回退
            </label>
          </div>
          <p>保留文件</p>
          {[
            "scripts/save.gd",
            "scenes/save_menu.tscn",
            "docs/存档与验收.md",
          ].map((file) => (
            <label className="studio-file-change" key={file}>
              <input
                type="checkbox"
                checked={keep.includes(file)}
                onChange={(e) =>
                  setKeep((old) =>
                    e.target.checked
                      ? [...old, file]
                      : old.filter((f) => f !== file),
                  )
                }
              />
              <code>{file}</code>
            </label>
          ))}
          <p className="attention">
            后续任务可能依赖这些改动。存在冲突时应保留当前内容，不强制覆盖。
          </p>
          <footer>
            <button onClick={() => setRollback(false)}>取消</button>
            <button
              className="primary"
              onClick={() => {
                if (method === "ai")
                  create(
                    `回退“${current.goal}”的效果，保留后续任务${keep.length ? `及 ${keep.join("、")}` : ""}。`,
                    [current.goal],
                  );
                else update({ ...current, status: "已回退", kept: keep });
                setRollback(false);
              }}
            >
              {method === "ai" ? "发起回退任务" : "预览回退结果"}
            </button>
          </footer>
        </Dialog>
      )}
    </div>
  );
}
