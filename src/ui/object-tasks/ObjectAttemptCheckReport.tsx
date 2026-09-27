import type { ObjectAttemptCheckReport as Report } from "../../shared/object-attempt-checks";

export function ObjectAttemptCheckReport({ report }: { report: Report }) {
  const { projectId, requestId, target } = report.request;
  return (
    <article aria-label="冻结尝试技术报告">
      <p>
        {report.time} · {report.passed ? "技术检查通过" : "技术检查未通过"}
      </p>
      <p>
        项目：{projectId} · 对象：{target.objectId}
      </p>
      <p>
        中修：{target.taskId} · 精修：{target.fineTaskId}
      </p>
      <p>
        Run：{target.runId} · 尝试：{target.attemptId}
      </p>
      <p>
        请求 ID：{requestId} · 检查器版本：{report.runnerVersion}
      </p>
      <dl style={{ overflowWrap: "anywhere" }}>
        <dt>尝试摘要</dt>
        <dd>{report.attemptDigest}</dd>
        <dt>输入摘要</dt>
        <dd>{report.inputDigest}</dd>
        <dt>输出摘要</dt>
        <dd>{report.outputDigest}</dd>
      </dl>
      <ul>
        {report.rules.map((rule) => (
          <li key={rule.id}>
            {rule.id === "checkpoint-integrity"
              ? "冻结内容完整性"
              : "改动代码结构"}
            ：{rule.passed ? "通过" : "未通过"}（检查 {rule.filesChecked}{" "}
            项，规则版本 {rule.version}）
            <ul>
              {rule.issues.map((issue, index) => (
                <li key={index}>{issue}</li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </article>
  );
}
