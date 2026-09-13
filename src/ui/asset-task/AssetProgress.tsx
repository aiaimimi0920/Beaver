import type { AssetTaskState } from "../../shared/asset-task";

const stageNames: Record<string, string> = {
  pending: "待开始",
  running: "进行中",
  completed: "已完成",
  suspended: "暂挂",
  deciding: "等待决策",
  adjusting: "需调整",
  checking: "需复核",
  failed: "失败",
};
const phaseNames: Record<string, string> = {
  producing: "制作中",
  adjusting: "调整中",
  ready: "已核验，等待交付",
};

export function AssetProgress({ asset }: { asset: AssetTaskState }) {
  const names = new Map(asset.stages.map((stage) => [stage.id, stage.name]));
  return (
    <section className="asset-progress" aria-label="实际制作阶段">
      <header>
        <strong>
          第 {asset.round} 轮 · {phaseNames[asset.phase] ?? asset.phase}
        </strong>
        <span>
          {asset.stages.filter((stage) => stage.status === "completed").length}{" "}
          / {asset.stages.length} 阶段完成
        </span>
      </header>
      {asset.recovery && (
        <p className="asset-notice">恢复说明：{asset.recovery}</p>
      )}
      {!asset.stages.length && (
        <p className="asset-caption">等待 AI 报告实际制作阶段与依赖。</p>
      )}
      <ol className="asset-stage-list">
        {asset.stages.map((stage) => (
          <li key={stage.id} data-status={stage.status}>
            <details
              open={[
                "running",
                "suspended",
                "deciding",
                "adjusting",
                "checking",
                "failed",
              ].includes(stage.status)}
            >
              <summary>
                <strong>{stage.name}</strong>
                <span>{stageNames[stage.status] ?? stage.status}</span>
              </summary>
              <p className="asset-caption">
                第 {stage.round} 轮 · {stage.id}
              </p>
              {stage.dependencies.length > 0 && (
                <p>
                  依赖：
                  {stage.dependencies
                    .map((id) => names.get(id) ?? id)
                    .join("、")}
                </p>
              )}
              {stage.objects.length > 0 && (
                <p>关联对象：{stage.objects.join("、")}</p>
              )}
              {stage.evidence && (
                <p className="asset-evidence">结果依据：{stage.evidence}</p>
              )}
            </details>
          </li>
        ))}
      </ol>
      {asset.checkpoint && (
        <details>
          <summary>最近保存点</summary>
          <p className="asset-caption">{asset.checkpoint}</p>
          <p className="asset-caption">
            恢复仅覆盖已保存场景；相对修改需要先核验。
          </p>
        </details>
      )}
    </section>
  );
}
