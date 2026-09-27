import { useEffect, useSyncExternalStore } from "react";
import type { ObjectTaskRecovery } from "./object-task-recovery";
import { ObjectTaskDispositionPanel } from "./ObjectTaskDispositionPanel";
import { ObjectTaskResumePanel } from "./ObjectTaskResumePanel";

const preparation = {
  pending: "待准备",
  preparing: "准备中",
  ready: "准备已完成",
  failed: "准备失败",
};
const writer = {
  active: "仍有活动写入者",
  stopRecorded: "已有停止与输出保存记录",
  unconfirmed: "尚未确认停止",
};
const content = {
  verified: "冻结内容已核验",
  invalid: "冻结内容无效",
  unchecked: "尚未检查",
};
const workspace = {
  matchesCheckpoint: "与保存的检查点一致",
  drifted: "已偏离保存的检查点",
  missing: "工作区缺失",
  invalid: "工作区无效",
  unchecked: "尚未检查",
};

export function ObjectTaskRecoveryPanel({
  session,
}: {
  session: ObjectTaskRecovery;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    const unsubscribe = window.beaver.subscribe(() => {
      void session.refresh();
    });
    void session.refresh();
    return () => {
      unsubscribe();
      session.cancel();
    };
  }, [session]);
  const busy = state.refreshing || state.phase === "submitting";
  const report = state.receipt?.result;
  const matches =
    state.view?.operation?.generation === state.receipt?.generation &&
    state.view?.reportMatchesRecords;
  return (
    <section className="object-task-record" aria-label="恢复核验">
      <h3>恢复核验</h3>
      <p>
        核验仅保存证据，不会恢复执行、释放对象或接受版本。文件状态以核验时为准。
      </p>
      {state.phase === "loading" && <p role="status">正在读取核验记录…</p>}
      {state.action === "verify" && (
        <p role="status">正在核验并保存结果，关闭窗口后请求仍可能完成。</p>
      )}
      {state.error && <p role="alert">{state.error}</p>}
      {state.phase === "ready" && !state.view && (
        <p>尚无工作区准备记录，暂不需要核验。</p>
      )}
      {state.view && (
        <p>
          当前准备状态：{preparation[state.view.preparationState]}；调度
          {state.view.paused ? "已暂停" : "未暂停"}。
        </p>
      )}
      {state.retryAvailable && (
        <p>有待确认的核验请求，重试将沿用原请求标识和目标。</p>
      )}
      {report && (
        <>
          <h4>最近核验报告（第 {state.receipt!.generation} 次）</h4>
          <dl>
            <dt>核验时的记录</dt>
            <dd>
              {report.recordsCurrent
                ? "核验期间保持一致"
                : "核验期间已变化或损坏"}
            </dd>
            <dt>写入者</dt>
            <dd>{writer[report.writerStatus]}</dd>
            <dt>冻结内容</dt>
            <dd>{content[report.contentStatus]}</dd>
            <dt>工作区</dt>
            <dd>{workspace[report.workspaceStatus]}</dd>
            <dt>核验时的暂停状态</dt>
            <dd>{report.paused ? "已暂停" : "未暂停"}</dd>
          </dl>
          {report.issues.length > 0 && (
            <ul aria-label="核验问题">
              {report.issues.map((issue, index) => (
                <li key={index}>{issue}</li>
              ))}
            </ul>
          )}
          <p>
            {!state.view
              ? "当前记录尚未确认，请刷新核验记录。"
              : matches
                ? "报告对应的持久化记录仍匹配；查询未重新检查文件。"
                : "报告已过期或已有后续核验，请重新核验当前记录。"}
          </p>
          <p>
            {state.view?.canDispose && matches
              ? "本次证据满足取消并保留成果的条件，执行前仍需重新核验。"
              : state.dispositionReceipt?.result?.status ===
                    "cancelledAndRetained" ||
                  state.dispositionReceipt?.result?.status ===
                    "cancelledAndWorkspaceRemoved"
                ? "处置已完成，核验报告作为历史保留。"
                : "当前证据不足以进行新的处置。"}
          </p>
        </>
      )}
      {state.view?.target.recoveryGeneration === Number.MAX_SAFE_INTEGER && (
        <p>核验代次已耗尽，保留现有报告供查看。</p>
      )}
      <div className="object-task-editor-actions">
        <button
          disabled={!session.canVerify()}
          onClick={() => {
            void session.verify();
          }}
        >
          {state.retryAvailable ? "重试同一核验请求" : "核验当前记录与工作区"}
        </button>
        <button
          disabled={busy}
          onClick={() => {
            void session.refresh();
          }}
        >
          刷新核验记录
        </button>
      </div>
      <ObjectTaskDispositionPanel session={session} state={state} />
      <ObjectTaskResumePanel session={session} />
    </section>
  );
}
