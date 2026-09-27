import type { ObjectTaskRecord } from "../../shared/object-tasks";
import type { ObjectTaskQueryState } from "../object-tasks/object-task-query";
import { objectTaskStatusLabels } from "../object-tasks/object-task-status";

export function ObjectProductionTasks({
  query,
  objectId,
  taskId,
  manufacture,
  select,
  openObject,
  execute,
  refresh,
}: {
  query: ObjectTaskQueryState;
  objectId?: string;
  taskId?: string;
  manufacture: boolean;
  select: (task: ObjectTaskRecord) => void;
  openObject: (objectId: string) => void;
  execute: (task: ObjectTaskRecord) => void;
  refresh: () => void;
}) {
  if (query.kind === "loading") return <p role="status">正在读取对象任务…</p>;
  if (query.kind === "error")
    return (
      <section role="alert">
        <p>{query.message}</p>
        <button onClick={refresh}>重试读取任务</button>
      </section>
    );
  const tasks = query.snapshot.tasks;
  const iterations = tasks
    .filter(
      (task) =>
        task.granularity === "medium" &&
        (!objectId || task.objectId === objectId),
    )
    .sort(
      (a, b) =>
        (a.position ?? 0) - (b.position ?? 0) || a.id.localeCompare(b.id),
    );
  const selected = iterations.find((task) => task.id === taskId);
  const stages = selected
    ? tasks
        .filter(
          (task) =>
            task.parentTaskId === selected.id && task.granularity === "fine",
        )
        .sort(
          (a, b) =>
            (a.position ?? 0) - (b.position ?? 0) || a.id.localeCompare(b.id),
        )
    : [];
  const parent =
    selected && tasks.find((task) => task.id === selected.parentTaskId);
  return (
    <section className="op-production-tasks" aria-label="对象制作迭代">
      <header>
        <h2>{objectId ? "当前对象的迭代" : "项目制作迭代"}</h2>
        <button onClick={refresh}>刷新任务</button>
      </header>
      {!iterations.length && (
        <p role="status">当前范围尚无制作任务；对象仍可独立查看。</p>
      )}
      <ol>
        {iterations.map((task) => (
          <li key={task.id}>
            <button
              aria-pressed={task.id === taskId}
              onClick={() => select(task)}
            >
              {task.title} · {objectTaskStatusLabels[task.status]}
            </button>
            <small>
              对象 {task.objectId} · 迭代 {task.runId}
            </small>
          </li>
        ))}
      </ol>
      {taskId && !selected && (
        <p role="status">所选迭代不在当前范围，请重新选择。</p>
      )}
      {manufacture && selected && (
        <article aria-label="当前制作任务">
          <h3>{selected.title}</h3>
          <p>
            对象 {selected.objectId} · 中修 {selected.id} · 迭代{" "}
            {selected.runId}
          </p>
          {parent && (
            <p>
              责任粗修：{parent.title}（{parent.id}）
            </p>
          )}
          <p>{selected.prompt}</p>
          <p>验收要求：{selected.acceptance || "尚未填写"}</p>
          <button onClick={() => openObject(selected.objectId!)}>
            返回此对象
          </button>
          <button onClick={() => execute(selected)}>查看执行记录与恢复</button>
          <h4>精修阶段</h4>
          {!stages.length && <p>尚无精修阶段。</p>}
          <ol>
            {stages.map((task) => (
              <li key={task.id}>
                <details>
                  <summary>
                    {task.title} · {objectTaskStatusLabels[task.status]}
                  </summary>
                  <p>
                    精修 {task.id} · 阶段 {task.stageId}
                  </p>
                  <p>{task.prompt}</p>
                  <p>验收要求：{task.acceptance || "尚未填写"}</p>
                  <p>
                    依赖：
                    {task.dependsOn
                      .map(
                        (id) =>
                          tasks.find((item) => item.id === id)?.title ?? id,
                      )
                      .join("、") || "无"}
                  </p>
                </details>
              </li>
            ))}
          </ol>
          <p>任务状态来自保存记录；执行输出、检查和发布结果请查看执行记录。</p>
        </article>
      )}
      {manufacture && !selected && iterations.length > 0 && (
        <p>选择一次迭代查看制作阶段。</p>
      )}
    </section>
  );
}
