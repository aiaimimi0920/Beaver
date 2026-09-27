import type { ObjectTaskRecovery } from "./object-task-recovery";
import { ObjectReworkImageSummary } from "./ObjectReworkImageSummary";
import { FeedbackRelocationSummary } from "./FeedbackRelocationSummary";

export function ObjectTaskResumePanel({
  session,
}: {
  session: ObjectTaskRecovery;
}) {
  const resume = session.resume;
  const state = resume.getSnapshot();
  return (
    <section aria-label="恢复执行">
      <h4>
        {state.confirmation?.advance
          ? "确认接受并推进"
          : state.confirmation?.rework
            ? "确认按反馈返工候选"
            : "重试失败或中断的执行"}
      </h4>
      <p>
        沿用最近保存的输出检查点，创建新的执行尝试；原执行与检查点继续保留。重新打开项目不会自动执行。
      </p>
      {state.error && <p role="alert">{state.error}</p>}
      {state.submitting && (
        <p role="status">正在核验并提交新尝试，关闭窗口后请求仍可能完成。</p>
      )}
      {state.receipt?.result?.status === "started" && (
        <p role="status">
          已创建尝试 {state.receipt.result.attemptId}
          。此回执只证明创建成功，当前进度请查看执行记录。
        </p>
      )}
      {state.receipt?.result?.status === "blocked" && (
        <p role="status">
          重新检查未通过，未创建新尝试。请处理核验问题后重新核验。
          {state.receipt.result.report.issues.join("；")}
        </p>
      )}
      {state.confirmation && (
        <div role="group" aria-label="确认恢复执行">
          <p>
            {state.confirmation.advance
              ? "确认后将接受当前细任务，并以已有成果启动下一细任务，可能产生新的费用。对象版本尚未发布。"
              : "确认后将启动新的模型执行，可能产生新的费用。已有成果将作为输入；此操作不会接受版本或跳过验收。"}
          </p>
          {state.confirmation.advance && (
            <p>
              下一细任务：{state.confirmation.advance.nextFineTaskId}
              ；人工验收说明：{state.confirmation.advance.acceptanceNote}
            </p>
          )}
          {state.confirmation.rework && (
            <p style={{ whiteSpace: "pre-wrap" }}>
              候选审阅：{state.confirmation.rework.reviewRequestId}； 原尝试：
              {state.confirmation.rework.attemptId}； 返工细任务：
              {state.confirmation.rework.fineTaskId}。
              已接受阶段保持不变，对象仍被当前任务占用。 修改意见：
              {state.confirmation.rework.feedback}
            </p>
          )}
          <ObjectReworkImageSummary image={state.confirmation.rework?.image} />
          <FeedbackRelocationSummary
            relocation={state.confirmation.rework?.relocation}
          />
          {state.confirmation.rework?.previewFrame && (
            <p>
              原始编号存档：{state.confirmation.rework.previewFrame.runId} /{" "}
              {state.confirmation.rework.previewFrame.frameId}
              。新尝试将读取该帧的
              PNG、编号意见和相机来源，修改前须重新核对位置。
            </p>
          )}
          <button
            onClick={() => {
              void resume.submit();
            }}
          >
            确认并启动新尝试
          </button>
          <button onClick={resume.dismiss}>返回</button>
        </div>
      )}
      {!state.confirmation && !state.retryAvailable && (
        <button disabled={!resume.canPrepare()} onClick={resume.prepare}>
          确认重试执行
        </button>
      )}
      {state.retryAvailable && (
        <button
          disabled={state.submitting || !session.resumeAvailable()}
          onClick={() => {
            void resume.submit();
          }}
        >
          重试同一恢复执行请求
        </button>
      )}
    </section>
  );
}
