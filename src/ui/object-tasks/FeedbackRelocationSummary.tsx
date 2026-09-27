import type { FeedbackRelocation } from "../../shared/preview-feedback-relocation";

export function FeedbackRelocationSummary({
  relocation,
}: {
  relocation?: FeedbackRelocation;
}) {
  if (!relocation) return null;
  return (
    <section aria-label="已确认的历史区域对应">
      <p>
        历史来源尝试：{relocation.sourceAttemptId}；原帧：
        {relocation.sourceFrame.runId} / {relocation.sourceFrame.frameId}
        。对应目标为本次反馈的原始编号帧，仅在该检查点确认。
      </p>
      <ol>
        {relocation.regions.map((region) => (
          <li key={region.sourceRegion}>
            历史区域 {region.sourceRegion + 1}：
            {region.status === "matched"
              ? `对应当前区域 ${region.targetRegion + 1}`
              : `无对应区域；${region.note}`}
          </li>
        ))}
      </ol>
    </section>
  );
}
