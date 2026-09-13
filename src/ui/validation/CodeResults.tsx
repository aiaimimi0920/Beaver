import type { ValidationRun } from "./types";

export function CodeResults({ run }: { run: ValidationRun }) {
  const report = run.code;
  if (!report)
    return (
      <p className="validation-notice">
        尚无有效 GUT 报告；缺少测试或运行失败均不会得到绿灯。
      </p>
    );
  return (
    <section>
      <h3>GUT {report.gutVersion}</h3>
      <p>
        通过 {report.passed} · 失败 {report.failed} · 跳过 {report.skipped} ·
        退出码 {report.exitCode ?? "未完成"}
      </p>
      <p className="muted">
        本次测试范围：{report.directories.join("、") || "未登记"}
        。通过仅代表此范围的实际用例与断言。
      </p>
      <div className="validation-table-wrap">
        <table className="validation-table">
          <thead>
            <tr>
              <th>用例</th>
              <th>状态 / 断言</th>
              <th>文件与问题</th>
            </tr>
          </thead>
          <tbody>
            {report.cases.map((test, i) => (
              <tr key={`${test.file}:${test.name}:${i}`}>
                <td>{test.name}</td>
                <td
                  className={
                    test.status === "pass" && test.assertions > 0
                      ? "validation-good"
                      : "validation-bad"
                  }
                >
                  {(
                    { pass: "通过", fail: "失败", skip: "跳过" } as Record<
                      string,
                      string
                    >
                  )[test.status] ?? test.status}{" "}
                  · {test.assertions}
                </td>
                <td>
                  <code>{test.file}</code>
                  {test.message && <pre>{test.message}</pre>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
