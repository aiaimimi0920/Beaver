import { useState } from "react";
import type { Feature, Project } from "../shared/types";
import {
  featureCategories,
  featureCategory,
  recommendedFeatures,
} from "../shared/game-design";
import { call, type Run } from "./api";
import { Icon } from "./Icon";
import {
  suggestedBlueprintFeatures,
  defaultBlueprint,
  type BlueprintDraft,
} from "../shared/project-blueprint";
import { BlueprintActions } from "./BlueprintActions";
import { PlannedFeatures } from "./PlannedFeatures";
import { NprPackagePanel } from "./NprPackagePanel";

export function FeaturesView({
  features,
  project,
  run,
  queued,
  draft,
  changeDraft,
  clearDraft,
}: {
  features: Feature[];
  project: Project;
  run: Run;
  queued: () => void;
  draft?: BlueprintDraft;
  changeDraft: (draft: BlueprintDraft) => void;
  clearDraft: () => void;
}) {
  const [view, setView] = useState("planning");
  const value =
    draft?.value ?? project.blueprint ?? defaultBlueprint(project.design);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");
  const [recommendedOnly, setRecommendedOnly] = useState(false);
  const hasDirection = !!(project.blueprint || project.design);
  const recommended = project.blueprint
    ? suggestedBlueprintFeatures(project.blueprint)
    : recommendedFeatures(project.design);
  const filtered = features.filter(
    (f) =>
      (!category || featureCategory(f) === category) &&
      (!hasDirection || !recommendedOnly || recommended.includes(f.id)) &&
      `${f.name} ${f.description}`
        .toLowerCase()
        .includes(search.trim().toLowerCase()),
  );
  return (
    <div className="features-page">
      {"__TAURI__" in window && (
        <NprPackagePanel key={project.id} project={project} run={run} />
      )}
      <div className="blueprint-tabs" role="tablist" aria-label="功能包工作区">
        {[
          { id: "planning", name: "功能包规划" },
          { id: "source", name: "源码包" },
        ].map((item) => (
          <button
            key={item.id}
            id={`packages-tab-${item.id}`}
            role="tab"
            aria-selected={view === item.id}
            aria-controls={`packages-panel-${item.id}`}
            tabIndex={view === item.id ? 0 : -1}
            onClick={() => setView(item.id)}
            onKeyDown={(e) => {
              if (["ArrowLeft", "ArrowRight"].includes(e.key)) {
                e.preventDefault();
                const next = item.id === "planning" ? "source" : "planning";
                setView(next);
                document.getElementById(`packages-tab-${next}`)?.focus();
              }
            }}
          >
            {item.name}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`packages-panel-${view}`}
        aria-labelledby={`packages-tab-${view}`}
      >
        {view === "planning" ? (
          <>
            <BlueprintActions
              project={project}
              draft={draft}
              clearDraft={clearDraft}
              run={run}
            />
            <PlannedFeatures
              value={value}
              features={features}
              change={(next) =>
                changeDraft({
                  value: next,
                  revision: draft?.revision ?? project.blueprintRevision ?? 0,
                })
              }
            />
          </>
        ) : (
          <>
            <div className="feature-filters">
              <input
                aria-label="搜索功能块"
                placeholder="搜索功能块"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              <select
                aria-label="功能块分类"
                value={category}
                onChange={(e) => setCategory(e.target.value)}
              >
                <option value="">全部分类</option>
                {featureCategories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
              {hasDirection && (
                <label className="check">
                  <input
                    type="checkbox"
                    checked={recommendedOnly}
                    onChange={(e) => setRecommendedOnly(e.target.checked)}
                  />
                  适合此项目
                </label>
              )}
            </div>
            <section className="feature-list">
              {filtered.map((f) => (
                <article className="feature-row" key={f.id}>
                  <span className="feature-icon">
                    <Icon name="features" />
                  </span>
                  <div>
                    <h3>
                      {f.name}
                      <small>v{f.version}</small>
                      {recommended.includes(f.id) && (
                        <small className="feature-recommended">推荐</small>
                      )}
                    </h3>
                    <p>{f.description}</p>
                  </div>
                  <button
                    onClick={() =>
                      void run(async () => {
                        await call("feature.add", {
                          projectId: project.id,
                          featureId: f.id,
                        });
                        queued();
                      })
                    }
                  >
                    接入 / 更新
                  </button>
                </article>
              ))}
              {!filtered.length && <p className="muted">没有匹配的功能块</p>}
            </section>
            <details className="inline-help">
              <summary>更新规则</summary>
              <p className="muted">
                功能块提供源码与接入约束，由 Codex
                适配现有项目；不是勾选后已经完成的玩法。更新时保留定制，经你认可后更新采用版本。模板仍仅在创建时应用。
              </p>
            </details>
          </>
        )}
      </div>
    </div>
  );
}
