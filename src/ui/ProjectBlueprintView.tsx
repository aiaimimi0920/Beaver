import { useState } from "react";
import type { Feature, Project } from "../shared/types";
import { campaignChoices, phases } from "../shared/blueprint-catalog";
import {
  defaultBlueprint,
  type BlueprintDraft,
  type CampaignKey,
  type PriorityKey,
  type ProjectBlueprint,
} from "../shared/project-blueprint";
import { PlannedFeatures } from "./PlannedFeatures";
import { BlueprintActions } from "./BlueprintActions";
import { Dialog, Field } from "./components";
import type { Run } from "./api";

export function ProjectBlueprintView({
  project,
  draft,
  changeDraft,
  clearDraft,
  features,
  run,
  openOverview,
}: {
  project: Project;
  draft?: BlueprintDraft;
  changeDraft: (draft: BlueprintDraft) => void;
  clearDraft: () => void;
  features: Feature[];
  run: Run;
  openOverview: () => void;
}) {
  const value =
    draft?.value ?? project.blueprint ?? defaultBlueprint(project.design);
  const [tab, setTab] = useState("basics");
  const [selected, setSelected] = useState<PriorityKey>("story");
  const [preview, setPreview] = useState(false);
  const [campaign, setCampaign] = useState<CampaignKey>();
  const [campaignDraft, setCampaignDraft] = useState({
    notes: "",
    channels: [] as string[],
  });
  const edit = (next: ProjectBlueprint) =>
    changeDraft({
      value: next,
      revision: draft?.revision ?? project.blueprintRevision ?? 0,
    });
  const phase = phases.find((p) => p.id === tab);
  const metric =
    phase?.metrics.find((m) => m.id === selected) ?? phase?.metrics[0];
  const tabs = [
    { id: "basics", name: "基本设定" },
    ...phases,
    { id: "launch", name: "发行准备" },
  ];
  const action = campaignChoices.find((c) => c.id === campaign);
  return (
    <section className="blueprint-page">
      <BlueprintActions
        project={project}
        draft={draft}
        clearDraft={clearDraft}
        run={run}
      />
      <div className="blueprint-tabs" role="tablist" aria-label="项目规划阶段">
        {tabs.map((item, i) => (
          <button
            key={item.id}
            id={`plan-tab-${item.id}`}
            role="tab"
            aria-selected={tab === item.id}
            aria-controls={`plan-panel-${item.id}`}
            tabIndex={tab === item.id ? 0 : -1}
            onKeyDown={(e) => {
              if (["ArrowLeft", "ArrowRight"].includes(e.key)) {
                e.preventDefault();
                const next =
                  tabs[
                    (i + (e.key === "ArrowRight" ? 1 : tabs.length - 1)) %
                      tabs.length
                  ]!;
                setTab(next.id);
                document.getElementById(`plan-tab-${next.id}`)?.focus();
              }
            }}
            onClick={() => setTab(item.id)}
          >
            {item.name}
          </button>
        ))}
      </div>
      <div
        className="blueprint-content"
        role="tabpanel"
        id={`plan-panel-${tab}`}
        aria-labelledby={`plan-tab-${tab}`}
      >
        {tab === "basics" ? (
          <>
            <div className="stage-toolbar">
              <span className="muted">基础设定已锁定，在总览中统一编辑。</span>
              <button onClick={openOverview}>前往总览编辑</button>
            </div>
            <details className="blueprint-help">
              <summary>功能规划（{value.plannedFeatures.length}）</summary>
              <PlannedFeatures
                value={value}
                change={edit}
                features={features}
              />
            </details>
          </>
        ) : phase && metric ? (
          <>
            <div className="stage-toolbar">
              <span className="muted">投入优先级 0–5 · 不是完成度</span>
              <button onClick={() => setPreview(true)}>预览阶段任务</button>
            </div>
            <div className="stage-board">
              <div className="stage-controls">
                {phase.metrics.map((m) => {
                  const inactive = m.id === "network" && !value.online.enabled;
                  return (
                    <div
                      className={`priority-row${metric.id === m.id ? " selected" : ""}`}
                      key={m.id}
                    >
                      <button
                        className="priority-name"
                        aria-label={`${m.name}含义`}
                        onClick={() => setSelected(m.id)}
                      >
                        {m.name}
                      </button>
                      <input
                        aria-label={`${m.name}投入`}
                        type="range"
                        min={0}
                        max={5}
                        step={1}
                        value={value.priorities[m.id]}
                        disabled={inactive}
                        onFocus={() => setSelected(m.id)}
                        onChange={(e) =>
                          edit({
                            ...value,
                            priorities: {
                              ...value.priorities,
                              [m.id]: Number(e.target.value),
                            },
                          })
                        }
                      />
                      <output>
                        {inactive
                          ? "单人不启用"
                          : `${value.priorities[m.id]} / 5`}
                      </output>
                    </div>
                  );
                })}
                <p className="muted blueprint-footnote">
                  0 不额外投入，3 标准关注，5 核心重点；必需基础不因设为 0
                  而省略。各项独立，可同时侧重。
                </p>
              </div>
              <aside className="metric-meaning" aria-live="polite">
                <strong>{metric.name}</strong>
                <p>{metric.meaning}</p>
                <span className="muted">未来交付</span>
                <p>{metric.output}</p>
                <span className="muted">验收目标</span>
                <p>{metric.acceptance}</p>
                <div className="planned-slot">待形成 · 当前未执行</div>
              </aside>
            </div>
          </>
        ) : (
          <div className="campaign-list">
            {campaignChoices.map((c) => (
              <article className="campaign-row" key={c.id}>
                <div>
                  <strong>{c.name}</strong>
                  <p>{c.meaning}</p>
                  <small className="muted">
                    {value.campaigns[c.id].notes ||
                    value.campaigns[c.id].channels.length
                      ? "已有规划草案 · 未执行"
                      : "尚未规划 · 未执行"}
                  </small>
                </div>
                <button
                  onClick={() => {
                    setCampaign(c.id);
                    setCampaignDraft(structuredClone(value.campaigns[c.id]));
                  }}
                >
                  规划
                </button>
              </article>
            ))}
          </div>
        )}
      </div>
      {preview && phase && (
        <Dialog
          title={`${phase.name}阶段 · 任务预览`}
          close={() => setPreview(false)}
        >
          <p className="muted">
            规划预览，不调用 Codex，不生成素材或修改项目。
          </p>
          <div className="planned-task-list">
            {phase.metrics.map((m) => (
              <div key={m.id}>
                <strong>
                  {m.name} ·{" "}
                  {m.id === "network" && !value.online.enabled
                    ? "单人不启用"
                    : `优先级 ${value.priorities[m.id]}`}
                </strong>
                <p>{m.output}</p>
              </div>
            ))}
          </div>
          <footer>
            <button onClick={() => setPreview(false)}>关闭预览</button>
          </footer>
        </Dialog>
      )}
      {action && campaign && (
        <Dialog
          title={`${action.name} · 规划`}
          close={() => setCampaign(undefined)}
        >
          <p>{action.meaning}</p>
          <p className="muted">
            未来交付：{action.outputs}。当前不会发送、发布、部署或付费。
          </p>
          <div className="campaign-channels">
            {action.channels.map((c) => (
              <label key={c} className="check">
                <input
                  type="checkbox"
                  checked={campaignDraft.channels.includes(c)}
                  onChange={(e) =>
                    setCampaignDraft({
                      ...campaignDraft,
                      channels: e.target.checked
                        ? [...campaignDraft.channels, c]
                        : campaignDraft.channels.filter((v) => v !== c),
                    })
                  }
                />
                {c}
              </label>
            ))}
          </div>
          <Field label="目标与约束">
            <textarea
              value={campaignDraft.notes}
              maxLength={3000}
              rows={5}
              placeholder="记录受众、素材、排期、预算上限或试玩内容范围"
              onChange={(e) =>
                setCampaignDraft({ ...campaignDraft, notes: e.target.value })
              }
            />
          </Field>
          <footer>
            <button onClick={() => setCampaign(undefined)}>取消</button>
            <button
              className="primary"
              onClick={() => {
                edit({
                  ...value,
                  campaigns: { ...value.campaigns, [campaign]: campaignDraft },
                });
                setCampaign(undefined);
              }}
            >
              保留草案
            </button>
          </footer>
        </Dialog>
      )}
    </section>
  );
}
