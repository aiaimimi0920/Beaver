import { label, shortId, time, verdictLabel, type ReleaseCheck } from "./types";

export function ReleaseStatus({
  check,
  openRun,
}: {
  check: ReleaseCheck;
  openRun: (id: string) => void;
}) {
  return (
    <section className="validation-release-status">
      <p>
        <strong className={check.ready ? "validation-good" : "validation-bad"}>
          {check.ready
            ? "候选检查通过，可正式导出"
            : "候选尚未通过正式发布检查"}
        </strong>
      </p>
      <p>
        固定快照{" "}
        <code title={check.snapshotId}>{shortId(check.snapshotId)}</code> ·{" "}
        {check.preset} · {time(check.createdAt)}
      </p>
      <p>
        本次画面诊断：
        {check.visualRequired
          ? "必须运行并全部通过"
          : "未要求画面诊断，仅要求代码验收"}
        。后续编辑和开关变更请建立新候选。
      </p>
      {check.integrityError && (
        <p className="validation-error">{check.integrityError}</p>
      )}
      {(check.missing ?? []).map((reason, i) => (
        <p className="validation-error" key={i}>
          缺少覆盖：{reason}
        </p>
      ))}
      <ul className="validation-release-items">
        {(check.items ?? []).map((item) => (
          <li
            key={item.runId}
            className={item.passed ? "validation-good" : "validation-bad"}
          >
            <button onClick={() => openRun(item.runId)}>
              {item.kind === "code"
                ? "GUT 代码验收"
                : `画面流程 ${shortId(item.flowId ?? item.runId)}`}
            </button>
            <span>
              {label(item.phase || item.status)} · {verdictLabel(item)}
            </span>
            {(item.integrityError || item.error) && (
              <p>{item.integrityError || item.error}</p>
            )}
          </li>
        ))}
      </ul>
      {check.excluded?.length > 0 && (
        <details>
          <summary>明确退役的流程</summary>
          <pre>{JSON.stringify(check.excluded, null, 2)}</pre>
        </details>
      )}
    </section>
  );
}
