import { useState } from "react";
import type { AssetTaskState } from "../../shared/asset-task";
import { call, type Run } from "../api";
import { ReferenceEditor } from "./ReferenceEditor";

const feedbackNames: Record<string, string> = {
  received: "已接收",
  delivering: "图像正在投递",
  waitingSwitch: "已送达，等待安全切换",
  acknowledged: "AI 已确认",
  deciding: "等待决策",
  executing: "执行中",
  checking: "检查中",
  completed: "已完成",
  failed: "失败，等待处理",
  cancelled: "已撤回",
  forwarded: "已转入后续任务",
  pendingVerification: "待核验，禁止直接重放",
};

export function FeedbackHistory({
  id,
  asset,
  run,
}: {
  id: string;
  asset: AssetTaskState;
  run: Run;
}) {
  const [expanded, setExpanded] = useState("");
  const [busy, setBusy] = useState("");
  const names = new Map(asset.stages.map((stage) => [stage.id, stage.name]));
  return (
    <section className="asset-feedback-history" aria-label="反馈处理记录">
      <h2>处理记录 · {asset.feedback.length} 条</h2>
      {!asset.feedback.length && (
        <p className="asset-caption">尚未提交修改要求。</p>
      )}
      <ol>
        {[...asset.feedback].reverse().map((feedback) => (
          <li key={`${feedback.sourceTaskId}:${feedback.id}`}>
            <header>
              <strong>
                {feedbackNames[feedback.status] ?? feedback.status}
              </strong>
              <span>第 {feedback.round} 轮</span>
            </header>
            <p className="asset-evidence">{feedback.text}</p>
            <p className="asset-caption">
              {feedback.timing === "now" ? "现在调整" : "完成后处理"} ·{" "}
              {new Date(feedback.createdAt).toLocaleString()}
            </p>
            {feedback.round > asset.round && (
              <p className="asset-caption">
                等待当前制作轮次完成与检查，尚未向 AI 交付修改内容。
              </p>
            )}
            {feedback.taskId !== id && (
              <p className="asset-notice">
                归入后续任务 {feedback.taskId}，交付与回退边界独立。
                <button
                  onClick={() =>
                    void run(() =>
                      call("assetTask.open", { id: feedback.taskId }),
                    )
                  }
                >
                  打开后续制作窗口
                </button>
              </p>
            )}
            {feedback.sourceTaskId !== id && (
              <p className="asset-caption">来源任务：{feedback.sourceTaskId}</p>
            )}
            <div className="asset-toolbar">
              <button
                aria-expanded={expanded === feedback.id}
                onClick={() =>
                  setExpanded(expanded === feedback.id ? "" : feedback.id)
                }
              >
                {expanded === feedback.id ? "收起证据" : "查看截图与处理证据"}
              </button>
              {feedback.status === "received" && !feedback.deliveredAt && (
                <button
                  disabled={!!busy}
                  onClick={() => {
                    setBusy(feedback.id);
                    void run(
                      () =>
                        call("assetTask.cancel", {
                          id,
                          feedbackId: feedback.id,
                        }),
                      "未投递的反馈已撤回。",
                    ).finally(() => setBusy(""));
                  }}
                >
                  撤回
                </button>
              )}
            </div>
            {expanded === feedback.id && (
              <div className="asset-feedback-evidence">
                <p className="asset-caption">反馈编号：{feedback.id}</p>
                <ReferenceEditor
                  taskId={id}
                  reference={feedback.reference}
                  annotations={feedback.annotations}
                  change={() => {}}
                  locked
                />
                {feedback.imageObservation && (
                  <p className="asset-evidence">
                    图像核对：{feedback.imageObservation}
                  </p>
                )}
                {feedback.impact && (
                  <p className="asset-evidence">影响分析：{feedback.impact}</p>
                )}
                {feedback.affectedStages.length > 0 && (
                  <p>
                    受影响阶段：
                    {feedback.affectedStages
                      .map((value) => names.get(value) ?? value)
                      .join("、")}
                  </p>
                )}
                {feedback.evidence && (
                  <p className="asset-evidence">
                    核验依据：{feedback.evidence}
                  </p>
                )}
                {feedback.checkpoint && (
                  <p className="asset-caption">
                    修改前保存点：{feedback.checkpoint}
                  </p>
                )}
                <ol className="asset-feedback-timeline">
                  {feedback.history.map((entry, index) => (
                    <li key={index}>
                      <span>
                        {new Date(entry.at).toLocaleTimeString()} ·{" "}
                        {feedbackNames[entry.status] ?? entry.status}
                      </span>
                      {entry.evidence && (
                        <p className="asset-evidence">{entry.evidence}</p>
                      )}
                    </li>
                  ))}
                </ol>
              </div>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}
