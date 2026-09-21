import type { DeliveryCandidate } from "../../shared/asset-delivery";
import type { AssetWork } from "../../shared/asset-work";
import { operationTerminal } from "../../shared/framework";
import type { Run } from "../api";
import { FrameworkConfiguration } from "./FrameworkConfiguration";
import { FrameworkEvidence } from "./FrameworkEvidence";
import { FrameworkInputs } from "./FrameworkInputs";
import { FrameworkReview } from "./FrameworkReview";
import { useFramework } from "./use-framework";

export function FrameworkPanel({
  id,
  status,
  candidate,
  work,
  run,
}: {
  id: string;
  status: string;
  candidate?: DeliveryCandidate;
  work?: AssetWork;
  run: Run;
}) {
  const { data, error, busy, action, start } = useFramework(id, run);
  if (!data) return <p role="status">{error || "正在读取工作流框架…"}</p>;
  const disabled =
    busy || data.operations.some((op) => !operationTerminal(op.status));
  return (
    <section className="asset-work" aria-label="工作流框架">
      <h3>工作流框架</h3>
      {error && <p role="status">{error}</p>}
      <FrameworkConfiguration
        key={`${id}-${data.configuration.revision}`}
        configuration={data.configuration}
        disabled={disabled}
        action={action}
      />
      <h4>插件准备</h4>
      {!data.configuration.plugins.length && <p>尚未配置插件适配器。</p>}
      {data.configuration.plugins.map((plugin) => (
        <div key={plugin.id}>
          <p>
            {plugin.host} · {plugin.id} · {plugin.version} / 宿主{" "}
            {plugin.hostVersion}
          </p>
          <div className="asset-toolbar">
            {(["probe", "install", "enable", "reload"] as const).map((op) => (
              <button
                key={op}
                disabled={disabled || status === "running" || !plugin[op]}
                onClick={() =>
                  void start({ kind: "plugin", plugin: plugin.id, action: op })
                }
              >
                {
                  {
                    probe: "探测",
                    install: "安装",
                    enable: "启用",
                    reload: "重载",
                  }[op]
                }
              </button>
            ))}
          </div>
        </div>
      ))}
      <p>
        人工插件操作前先中止执行中的任务。安装后重新探测，版本、启用、可调用性及重启要求全部满足才记录
        ready。
      </p>
      {candidate && (
        <FrameworkReview
          key={candidate.id}
          candidate={candidate}
          revision={data.configuration.revision}
          disabled={disabled}
          action={action}
          start={start}
        />
      )}
      <FrameworkInputs work={work} disabled={disabled} start={start} />
      <FrameworkEvidence data={data} busy={busy} action={action} />
    </section>
  );
}
