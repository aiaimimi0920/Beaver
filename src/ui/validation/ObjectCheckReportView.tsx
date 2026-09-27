import { useEffect, useMemo, useSyncExternalStore } from "react";
import type { ObjectAttemptCheckRequest } from "../../shared/object-attempt-checks";
import { call } from "../api";
import { ObjectAttemptCheckReport } from "../object-tasks/ObjectAttemptCheckReport";
import { ObjectCheckReportQuery } from "./object-check-report";

export function ObjectCheckReportContent({
  session,
  openManufacture,
  openWorkbench,
}: {
  session: ObjectCheckReportQuery;
  openManufacture: (request: ObjectAttemptCheckRequest) => void;
  openWorkbench: () => void;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    void session.refresh();
    const unsubscribe = window.beaver.subscribe(() => void session.refresh());
    return () => {
      unsubscribe();
      session.cancel();
    };
  }, [session]);
  return (
    <section className="validation-page" aria-label="对象尝试技术报告">
      <h1>测试 · 对象尝试技术报告</h1>
      <p>
        以下与制造记录引用同一份持久化报告。仅记录检查时的结果，不代表当前工作区状态、阶段接受、对象发布或游戏测试通过。
      </p>
      <p>指定请求：{session.request.requestId}</p>
      <div className="validation-toolbar">
        <button disabled={state.busy} onClick={() => void session.refresh()}>
          重新读取报告
        </button>
        <button onClick={() => openManufacture(session.request)}>
          返回对应制造记录
        </button>
        <button onClick={openWorkbench}>打开游戏测试工作台</button>
      </div>
      {state.busy && <p role="status">正在读取历史技术报告…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.source && (
        <section aria-label="冻结报告来源">
          <p>冻结阶段：{state.source.stageId}</p>
          <h2>引用此报告的候选审阅</h2>
          <p>
            仅显示明确引用本检查请求的持久化审阅，候选审阅不代表接受或发布。
          </p>
          {!state.source.candidateReviews.length && (
            <p>暂无候选审阅引用此报告。</p>
          )}
          <ul>
            {state.source.candidateReviews.map((review) => (
              <li key={review.requestId}>
                <p>审阅请求：{review.requestId}</p>
                <p>来源摘要：{review.sourceDigest}</p>
                <p>输出摘要：{review.outputDigest}</p>
              </li>
            ))}
          </ul>
        </section>
      )}
      {state.report && <ObjectAttemptCheckReport report={state.report} />}
    </section>
  );
}

export function ObjectCheckReportView({
  projectId,
  request,
  ...navigation
}: {
  projectId: string;
  request: ObjectAttemptCheckRequest;
  openManufacture: (request: ObjectAttemptCheckRequest) => void;
  openWorkbench: () => void;
}) {
  const session = useMemo(
    () => new ObjectCheckReportQuery(projectId, request, call),
    [projectId, request],
  );
  return <ObjectCheckReportContent session={session} {...navigation} />;
}
