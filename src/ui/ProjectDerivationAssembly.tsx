import type { ProjectDerivationSession } from "./project-derivation-session";

export function ProjectDerivationAssembly({
  session: s,
  busy,
  perform,
  openProject,
}: {
  session: ProjectDerivationSession;
  busy: boolean;
  perform: (work: () => Promise<void>) => Promise<void>;
  openProject?: (id: string) => Promise<void>;
}) {
  const pathsReady = !!s.preparation.trim() && !!s.destination.trim();
  const retryRegistration = s.mode === "registered" || s.registrationAttempted;
  const receipt = s.assembly?.assembly;
  return (
    <>
      {!retryRegistration && (
        <>
          <button
            disabled={
              busy ||
              !s.prepared ||
              !s.destination.trim() ||
              !!s.assembly ||
              s.destination.trim() === s.failedAssembly
            }
            onClick={() => void perform(s.assemble)}
          >
            组装到新目录
          </button>
          <button
            disabled={busy || !pathsReady || !!s.assembly?.activated}
            onClick={() => void perform(s.inspectAssembly)}
          >
            核验已有组装副本
          </button>
        </>
      )}
      {s.failedAssembly && !s.assembly && (
        <p role="status">
          组装结果未知或失败，保留 {s.failedAssembly}
          。先核验已有回执；不完整时换一个全新组装目录，不能覆盖重试。
        </p>
      )}
      {receipt && (
        <div role="status">
          <p>
            组装已核验：{s.assembly?.activated ? "已启用" : "尚未启用或登记"}。
          </p>
          <p>新项目 ID：{receipt.projectId}</p>
          <p>
            最终项目目录：{receipt.binding}。绝对路径已绑定，组装后请勿移动。
          </p>
          <p>
            已中断历史任务：{receipt.tasksInterrupted}；已转换会话路径：
            {receipt.sessionPathsRewritten}。
          </p>
          <p>准备摘要：{receipt.preparationSha256}</p>
        </div>
      )}
      {s.assembly && !s.assembly.activated && !retryRegistration && (
        <>
          {s.confirm ? (
            <>
              <p role="alert">
                确认启用新身份副本 {receipt?.binding}
                ？不会覆盖源项目，也不会自动启动历史任务；宿主登记仍须单独确认。
              </p>
              <button disabled={busy} onClick={() => s.setConfirm(false)}>
                取消组装启用
              </button>
              <button disabled={busy} onClick={() => void perform(s.activate)}>
                确认启用组装副本
              </button>
            </>
          ) : (
            <button disabled={busy} onClick={() => s.setConfirm(true)}>
              启用组装副本
            </button>
          )}
        </>
      )}
      {(s.assembly?.activated || retryRegistration) &&
        !s.registration?.runtimeReady && (
          <>
            <p className="muted">
              登记会将已启用的副本加入当前
              Beaver，并恢复项目运行时；历史任务保持中断。响应丢失或恢复失败请使用相同路径重试登记，不再核验旧文件清单。
            </p>
            <button
              disabled={busy || !pathsReady}
              onClick={() => void perform(s.register)}
            >
              登记并恢复运行时
            </button>
          </>
        )}
      {s.registration && (
        <p role={s.registration.runtimeReady ? "status" : "alert"}>
          组装副本已登记。
          {s.registration.runtimeReady
            ? "项目运行时已恢复。"
            : `运行时恢复失败，可重试登记：${s.registration.runtimeError ?? "未知错误"}`}
        </p>
      )}
      {s.registration?.schedulerWakeError && (
        <p role="alert">
          项目已登记，但调度通知失败：{s.registration.schedulerWakeError}
        </p>
      )}
      {s.registration?.runtimeReady && openProject && (
        <button
          disabled={busy}
          onClick={() =>
            void perform(() => openProject(s.registration!.projectId))
          }
        >
          打开派生项目
        </button>
      )}
    </>
  );
}
