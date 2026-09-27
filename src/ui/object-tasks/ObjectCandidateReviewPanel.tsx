import { useEffect, useSyncExternalStore } from "react";
import type { ObjectCandidateReview } from "./object-candidate-review";
import type { ObjectAttemptChecks } from "./object-attempt-checks";
import type { ObjectTaskExecution } from "./object-task-execution";
import type { ObjectExecution } from "../../shared/object-attempts";
import { ObjectCandidateReworkPanel } from "./ObjectCandidateReworkPanel";
import { ObjectPublicationPanel } from "./ObjectPublicationPanel";

const blockers: Record<string, string> = {
  FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED: "最后细任务尚未人工接受",
  FEEDBACK_REVIEW_UNAVAILABLE: "旧版记录：生成时反馈处置检查尚未接入",
  PUBLICATION_NOT_IMPLEMENTED: "旧版记录：生成时受管发布尚未接入",
  PUBLICATION_CONFIRMATION_REQUIRED:
    "需打开发布预览，核对文件归属与返工反馈并提交人工接受决定",
  TECHNICAL_CHECK_FAILED: "本次重新检查发现技术问题",
  ACCEPTED_VERSION_DRIFT: "当前接受版本已偏离领取时版本",
  OBJECT_REVISION_DRIFT: "对象登记已偏离领取时 revision",
  REFERENCE_CLOSURE_REVIEW_REQUIRED: "当前固定引用与制作基线闭包需要重新核对",
};

export function ObjectCandidateReviewPanel({
  session,
  checks,
  execution,
  attempt,
}: {
  session: ObjectCandidateReview;
  checks: ObjectAttemptChecks;
  execution?: ObjectTaskExecution;
  attempt?: ObjectExecution["attempt"];
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const technical = useSyncExternalStore(
    checks.subscribe,
    checks.getSnapshot,
    checks.getSnapshot,
  );
  useEffect(() => {
    void session.refresh();
    return session.cancel;
  }, [session]);
  const latest = technical.reports.at(-1);
  return (
    <section aria-label="对象整体候选审阅">
      <h4>对象整体候选审阅</h4>
      <p>
        生成审阅记录会冻结最后输出、阶段定义和原始制作基线的差异，并重新检查内容。记录仅反映生成时状态；查看或重开记录不会接受、发布或释放对象占用。
      </p>
      <button
        disabled={
          state.busy ||
          (!state.retry &&
            (technical.busy || technical.retry || !latest?.passed))
        }
        onClick={() => {
          void session.prepare(latest?.request.requestId);
        }}
      >
        {state.retry ? "重试同一候选审阅请求" : "生成整体候选审阅记录"}
      </button>
      <button
        disabled={state.busy}
        onClick={() => {
          void session.refresh();
        }}
      >
        刷新候选审阅历史
      </button>
      {state.busy && <p role="status">正在读取或准备候选审阅…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {!state.busy && !state.reports.length && (
        <p>尚无整体候选审阅记录，请先完成上方技术检查。</p>
      )}
      {state.reports.map((report) => (
        <details key={report.request.requestId}>
          <summary>{report.time} · 整体候选（冻结时尚未接受或发布）</summary>
          <p>
            审阅 ID：{report.request.requestId} · 技术报告：
            {report.request.checkRequestId}
          </p>
          <p style={{ overflowWrap: "anywhere" }}>
            冻结输出摘要：{report.outputDigest} · 审阅来源摘要：
            {report.sourceDigest}
          </p>
          <p>制作基线：{report.baselineVersionId ?? "空基线"}</p>
          <p>
            领取时接受版本：{report.acceptedVersionIdAtClaim ?? "无"} ·
            审阅时接受版本：{report.acceptedVersionIdAtReview ?? "无"}
          </p>
          <p>
            对象 revision：领取时 {report.objectRevisionAtClaim} / 审阅时{" "}
            {report.objectRevisionAtReview}
          </p>
          <h5>冻结阶段定义</h5>
          <ol>
            {report.stages.map((stage) => (
              <li key={stage.taskId}>
                {stage.title} · {stage.taskId} · revision {stage.revision} ·{" "}
                {stage.status === "accepted" ? "已接受" : "等待人工接受"}
                <p style={{ whiteSpace: "pre-wrap" }}>
                  {stage.acceptance || "未填写验收要求"}
                </p>
              </li>
            ))}
          </ol>
          <h5>固定引用版本</h5>
          {report.references.length ? (
            <ul>
              {report.references.map((ref) => (
                <li key={ref.objectId}>
                  {ref.objectId} / {ref.versionId}
                </li>
              ))}
            </ul>
          ) : (
            <p>制作基线没有固定引用。</p>
          )}
          <h5>相对原始制作基线的完整文件清单</h5>
          <ul>
            {report.files.map((file) => (
              <li key={file.path} style={{ overflowWrap: "anywhere" }}>
                {file.path} ·{" "}
                {file.before === file.after
                  ? "未变更"
                  : !file.before
                    ? "新增"
                    : !file.after
                      ? "删除"
                      : "修改"}
                {file.reference ? " · 固定引用" : ""} · 登记归属：
                {file.owners.join(", ") || "未登记"}
                <p>
                  基线 SHA-256：{file.before ?? "无"} / 候选 SHA-256：
                  {file.after ?? "无"}
                </p>
              </li>
            ))}
          </ul>
          <h5>本次技术复查</h5>
          <ul>
            {report.rules.map((rule) => (
              <li key={rule.id}>
                {rule.id}：{rule.passed ? "通过" : "未通过"}
                <ul>
                  {rule.issues.map((issue, index) => (
                    <li key={index}>{issue}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
          <h5>审阅生成时尚未满足的条件（历史证据）</h5>
          {report.schemaVersion === 1 && (
            <p>
              此为旧版冻结审阅，能力限制仅描述记录生成时状态。当前可用的发布流程见下方；打开预览后仍需重新核验和人工确认。
            </p>
          )}
          <ul>
            {report.blockers.map((blocker, index) => (
              <li key={index}>{blockers[blocker] ?? blocker}</li>
            ))}
          </ul>
          {execution && attempt && (
            <ObjectCandidateReworkPanel
              key={report.request.requestId}
              session={execution}
              attempt={attempt}
              report={report}
            />
          )}
          {execution && (
            <ObjectPublicationPanel
              session={execution.publicationFor(report)}
            />
          )}
        </details>
      ))}
    </section>
  );
}
