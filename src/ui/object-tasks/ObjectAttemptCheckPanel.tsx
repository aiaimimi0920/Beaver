import { useContext, useEffect, useSyncExternalStore } from "react";
import type { ObjectAttemptChecks } from "./object-attempt-checks";
import { ObjectAttemptCheckReport } from "./ObjectAttemptCheckReport";
import { ObjectCheckNavigation } from "../validation/object-check-navigation";

export function ObjectAttemptCheckPanel({
  session,
  selectedRequestId,
}: {
  session: ObjectAttemptChecks;
  selectedRequestId?: string;
}) {
  const openReport = useContext(ObjectCheckNavigation);
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    void session.refresh();
    return session.cancel;
  }, [session]);
  return (
    <section aria-label="冻结尝试技术检查">
      <h4>技术检查</h4>
      <p>
        检查冻结输入、输出和初始基线的内容完整性，以及输出相对初始基线改动的代码结构。报告仅记录检查时的结果，不代表当前工作区状态或验收通过。
      </p>
      <button
        disabled={state.busy}
        onClick={() => {
          void session.run();
        }}
      >
        {state.retry ? "重试同一检查请求" : "发起新检查"}
      </button>
      <button
        disabled={state.busy}
        onClick={() => {
          void session.refresh();
        }}
      >
        刷新历史报告
      </button>
      {state.busy && <p role="status">正在读取或检查冻结内容…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {!state.busy && !state.reports.length && <p>暂无技术报告。</p>}
      <ul>
        {state.reports.map((report) => (
          <li key={report.request.requestId}>
            <details
              open={selectedRequestId === report.request.requestId || undefined}
            >
              <summary>
                {report.time} ·{" "}
                {report.passed ? "技术检查通过" : "技术检查未通过"}
              </summary>
              <ObjectAttemptCheckReport report={report} />
              {openReport && (
                <button onClick={() => openReport(report.request)}>
                  在测试页查看此报告
                </button>
              )}
            </details>
          </li>
        ))}
      </ul>
    </section>
  );
}
