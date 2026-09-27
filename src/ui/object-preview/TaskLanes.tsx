import { useState } from "react";
import { Icon } from "../Icon";
import { objectById, type DemoStatus } from "./mock-objects";
import { demoTasks, type DemoTask } from "./mock-tasks";
import { ObjectArt } from "./ObjectArt";
import { Status, type DemoAction } from "./PreviewControls";

const columns: { title: string; statuses: DemoStatus[]; tone: string }[] = [
  { title: "等待开始", statuses: ["排队中", "等待依赖"], tone: "muted" },
  { title: "执行中", statuses: ["执行中"], tone: "info" },
  { title: "需要关注", statuses: ["待验收", "执行失败"], tone: "attention" },
  { title: "已验收", statuses: ["已验收"], tone: "success" },
];

function TaskCard({
  task,
  openTask,
  openCreation,
}: {
  task: DemoTask;
  openTask: (id: string) => void;
  openCreation: (id: string, stage?: number) => void;
}) {
  const object = task.objectId ? objectById(task.objectId) : null;
  return (
    <article
      className={`op-task-card ${task.id === "T-201" ? "op-task-featured" : ""}`}
    >
      <button className="op-task-card-select" onClick={() => openTask(task.id)}>
        <div className="op-task-card-meta">
          <span className={`op-level op-level-${task.level}`}>
            {task.level}
          </span>
          <span className="op-mono">{task.id}</span>
        </div>
        <h3>{task.title}</h3>
        {task.id === "T-201" && (
          <div className="op-task-card-art">
            <ObjectArt kind="character" />
          </div>
        )}
        <p>{task.detail}</p>
        <Status value={task.status} />
        {task.status === "执行中" && (
          <div className="op-progress">
            <span style={{ width: `${task.progress}%` }} />
          </div>
        )}
        {task.parentId && (
          <small className="op-parent-label">
            ↳ {task.parentId} ·{" "}
            {task.level === "精修" ? "对象任务的子任务" : "主任务的子任务"}
          </small>
        )}
      </button>
      {object && (
        <button
          className="op-card-object"
          onClick={() => openCreation(object.id, task.stage)}
        >
          <Icon name="features" />
          <span>{object.name}</span>
          <span>↗</span>
        </button>
      )}
    </article>
  );
}

export function TaskLanes({
  openTask,
  openCreation,
  action,
  tasks = demoTasks,
}: {
  openTask: (id: string) => void;
  openCreation: (id: string, stage?: number) => void;
  action: DemoAction;
  tasks?: DemoTask[];
}) {
  const [scope, setScope] = useState("all");
  const [level, setLevel] = useState("全部层级");
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState(true);
  const visible = tasks.filter((task) => {
    const inScope =
      scope === "all" ||
      (scope === "T-100"
        ? task.id !== "T-105"
        : task.id === scope || task.parentId === scope);
    return (
      inScope &&
      (level === "全部层级" || task.level === level) &&
      `${task.title} ${task.id}`.toLowerCase().includes(search.toLowerCase())
    );
  });
  return (
    <section className="op-page">
      <header className="op-page-heading">
        <h1>
          任务 <span>{demoTasks.length}</span>
        </h1>
        <button
          className="primary"
          onClick={() =>
            action("新建任务", "粗修 · 自动拆分为对象任务与阶段任务")
          }
        >
          <Icon name="add" />
          新建任务
        </button>
      </header>
      <div className="op-tasks-layout">
        <aside className="op-task-tree" aria-label="任务层级">
          <button
            className={scope === "all" ? "is-active" : ""}
            onClick={() => setScope("all")}
          >
            <Icon name="tasks" />
            <strong>全部任务</strong>
            <span>{tasks.length}</span>
          </button>
          <div className="op-tree-heading">
            <span>任务分解</span>
            <button
              aria-label={expanded ? "收起任务树" : "展开任务树"}
              aria-expanded={expanded}
              onClick={() => setExpanded(!expanded)}
            >
              {expanded ? "−" : "+"}
            </button>
          </div>
          <button
            className={`op-tree-root ${scope === "T-100" ? "is-active" : ""}`}
            onClick={() => setScope("T-100")}
          >
            <Icon name="layers" />
            <span>
              教室里的舞蹈<small>T-100 · 粗修</small>
            </span>
          </button>
          {expanded && (
            <div className="op-tree-children">
              {tasks
                .filter((task) => task.parentId === "T-100")
                .map((task) => (
                  <div key={task.id}>
                    <button
                      className={scope === task.id ? "is-active" : ""}
                      onClick={() => setScope(task.id)}
                    >
                      <Icon
                        name={task.objectId === "O-004" ? "code" : "features"}
                      />
                      <span>
                        {objectById(task.objectId!).name}
                        <small>{task.id} · 中修</small>
                      </span>
                    </button>
                    {tasks
                      .filter((child) => child.parentId === task.id)
                      .map((child) => (
                        <button
                          key={child.id}
                          className="op-tree-fine"
                          onClick={() => openTask(child.id)}
                        >
                          <span className="op-tree-dot" />
                          <span>{child.title}</span>
                        </button>
                      ))}
                  </div>
                ))}
            </div>
          )}
          <div className="op-tree-heading">
            <span>独立修改</span>
            <span>1</span>
          </div>
          <button
            className={scope === "T-105" ? "is-active" : ""}
            onClick={() => setScope("T-105")}
          >
            <Icon name="tasks" />
            <span>
              增加蓝色发饰<small>中修 · 排队中</small>
            </span>
          </button>
          <button
            className="op-decompose"
            onClick={() => action("调整任务拆分", "T-100 · 教室里的舞蹈")}
          >
            <Icon name="settings" />
            调整拆分
          </button>
        </aside>
        <div className="op-task-board">
          <div className="op-goal-strip">
            <div>
              <span className="op-level">主任务</span>
              <strong>让澪在午后的教室里跳舞</strong>
              <p>角色与教室独立制作 → 验收后整合舞蹈演出</p>
            </div>
            <button onClick={() => openTask("T-100")}>查看目标 ↗</button>
          </div>
          <div className="op-board-toolbar">
            <div className="op-filters" aria-label="任务精细度">
              {["全部层级", "粗修", "中修", "精修"].map((value) => (
                <button
                  key={value}
                  aria-pressed={level === value}
                  onClick={() => setLevel(value)}
                >
                  {value}
                </button>
              ))}
            </div>
            <input
              aria-label="搜索任务"
              placeholder="搜索任务…"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
          </div>
          <div className="op-lanes">
            {columns.map((column) => {
              const tasks = visible.filter((task) =>
                column.statuses.includes(task.status),
              );
              return (
                <section
                  className="op-lane"
                  key={column.title}
                  aria-label={column.title}
                >
                  <header>
                    <span className={`op-lane-dot op-${column.tone}`} />
                    <h2>{column.title}</h2>
                    <span>{tasks.length}</span>
                  </header>
                  <div className="op-lane-scroll">
                    {tasks.map((task) => (
                      <TaskCard
                        key={task.id}
                        task={task}
                        openTask={openTask}
                        openCreation={openCreation}
                      />
                    ))}
                    {!tasks.length && (
                      <p className="op-lane-empty">暂无匹配任务</p>
                    )}
                  </div>
                </section>
              );
            })}
          </div>
          <footer className="op-board-legend">
            <span>粗修 → 中修 → 精修</span>
            <span>同一对象的修改串行排队 · 独立对象可并行</span>
          </footer>
        </div>
      </div>
    </section>
  );
}
