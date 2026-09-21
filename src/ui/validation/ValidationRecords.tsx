import type { Task } from "../../shared/types";
import { Icon } from "../Icon";
import { ReleaseStatus } from "./ReleaseStatus";
import { label, shortId, time, type ValidationData } from "./types";

export function ValidationRecords({
  page,
  data,
  tasks,
  openTask,
  openRun,
  exportGame,
}: {
  page: "coverage" | "releases";
  data: ValidationData;
  tasks: Task[];
  openTask: (id: string) => void;
  openRun: (id: string) => void;
  exportGame: (releaseId?: string) => void;
}) {
  if (page === "releases")
    return (
      <section className="validation-records validation-releases">
        <header className="validation-record-heading">
          <div>
            <h2>发布记录</h2>
            <p className="muted">查看固定候选的检查结果与导出记录。</p>
          </div>
          <button onClick={() => exportGame()}>
            <Icon name="import" /> 发布检查 / 导出
          </button>
        </header>
        {data.releases.length ? (
          data.releases.map((check) => (
            <details key={check.id}>
              <summary>
                {time(check.createdAt)} · {check.preset} ·{" "}
                <span
                  className={
                    check.ready ? "validation-good" : "validation-attention"
                  }
                >
                  {check.ready ? "通过" : "待处理"}
                </span>
              </summary>
              <ReleaseStatus check={check} openRun={openRun} />
              <button onClick={() => exportGame(check.id)}>
                使用此固定候选导出
              </button>
            </details>
          ))
        ) : (
          <div className="validation-empty">
            <Icon name="import" />
            <h3>还没有发布记录</h3>
            <p>准备导出时，在这里发起发布检查或内部导出。</p>
          </div>
        )}
      </section>
    );
  return (
    <section className="validation-records validation-coverage">
      <header className="validation-record-heading">
        <div>
          <h2>任务覆盖</h2>
          <p className="muted">追踪任务的画面证据、缺失项与自动修复。</p>
        </div>
        <span className="muted">{data.coverage.length} 个任务</span>
      </header>
      {data.coverage.map((item) => (
        <article key={item.taskId}>
          <button onClick={() => openTask(item.taskId)}>
            {tasks.find((task) => task.id === item.taskId)?.title ??
              item.taskId}
          </button>
          <span
            className={
              ["missing", "failed"].includes(item.status)
                ? "validation-bad"
                : "muted"
            }
          >
            {label(item.status)}
          </span>
          {item.reason && <p>{item.reason}</p>}
          <div className="validation-toolbar">
            {item.runIds?.map((id) => (
              <button key={id} onClick={() => openRun(id)}>
                查看运行 {shortId(id)}
              </button>
            ))}
          </div>
        </article>
      ))}
      {data.repairDecisions.length > 0 && <h3>自动修复</h3>}
      {data.repairDecisions.map((decision) => (
        <article key={decision.runId}>
          <button onClick={() => openRun(decision.runId)}>
            代码修复 {shortId(decision.runId)}
          </button>
          <span>
            {label(decision.status)} · {decision.reason}
          </span>
          {decision.taskId && (
            <button onClick={() => openTask(decision.taskId!)}>
              关联修复任务
            </button>
          )}
        </article>
      ))}
      {!data.coverage.length && !data.repairDecisions.length && (
        <div className="validation-empty">
          <Icon name="tasks" />
          <h3>还没有任务覆盖记录</h3>
          <p>任务交付后会显示对应的画面覆盖情况。</p>
        </div>
      )}
    </section>
  );
}
