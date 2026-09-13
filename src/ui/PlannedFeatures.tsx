import { useState } from "react";
import type { Feature } from "../shared/types";
import { planningPackages, filterPackages } from "../shared/function-packages";
import {
  suggestedBlueprintFeatures,
  type ProjectBlueprint,
} from "../shared/project-blueprint";

export function PlannedFeatures({
  value,
  change,
  features,
}: {
  value: ProjectBlueprint;
  change: (value: ProjectBlueprint) => void;
  features: Feature[];
}) {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");
  const [kind, setKind] = useState("");
  const [selectedOnly, setSelectedOnly] = useState(false);
  const [detailId, setDetailId] = useState("");
  const packages = planningPackages(features);
  const filtered = filterPackages(
    packages,
    search,
    category,
    kind,
    selectedOnly ? value.plannedFeatures : undefined,
  );
  const detail = packages.find((p) => p.id === detailId);
  const setSelected = (ids: string[]) =>
    change({ ...value, plannedFeatures: ids });
  const add = (ids: string[]) =>
    setSelected([...new Set([...value.plannedFeatures, ...ids])]);
  const missing = value.plannedFeatures.filter(
    (id) => !packages.some((p) => p.id === id),
  );
  return (
    <section className="package-planner" aria-label="功能包规划目录">
      <div className="feature-picker-heading">
        <span role="status">
          {packages.length} 个功能包 · 已选 {value.plannedFeatures.length}
        </span>
        <button
          onClick={() =>
            add(
              suggestedBlueprintFeatures(value).filter((id) =>
                features.some((f) => f.id === id),
              ),
            )
          }
        >
          加入推荐
        </button>
        <button
          disabled={!filtered.length}
          onClick={() => add(filtered.map((p) => p.id))}
        >
          全选当前
        </button>
        <button onClick={() => setSelected([])}>清空</button>
      </div>
      <div className="package-filters">
        <input
          aria-label="搜索功能包"
          placeholder="搜索名称、用途"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <select
          aria-label="功能包分组"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
        >
          <option value="">全部分组</option>
          {[...new Set(packages.map((p) => p.category))].map((c) => (
            <option key={c}>{c}</option>
          ))}
        </select>
        <select
          aria-label="功能包来源"
          value={kind}
          onChange={(e) => setKind(e.target.value)}
        >
          <option value="">全部来源</option>
          <option value="planning">
            能力规划 · {packages.filter((p) => p.kind === "planning").length}
          </option>
          <option value="source">附带源码 · {features.length}</option>
        </select>
        <label className="check">
          <input
            type="checkbox"
            checked={selectedOnly}
            onChange={(e) => setSelectedOnly(e.target.checked)}
          />
          仅已选
        </label>
      </div>
      <div className="package-picker" aria-label="可选功能包">
        {filtered.map((p) => (
          <div
            className={`package-choice${detailId === p.id ? " selected" : ""}`}
            key={p.id}
          >
            <label className="check">
              <input
                type="checkbox"
                aria-label={`规划 ${p.name}`}
                checked={value.plannedFeatures.includes(p.id)}
                onChange={(e) => {
                  setDetailId(p.id);
                  if (e.target.checked) add([p.id]);
                  else
                    setSelected(
                      value.plannedFeatures.filter((id) => id !== p.id),
                    );
                }}
              />
              <span>{p.name}</span>
            </label>
            <small className="muted">
              {p.kind === "planning" ? "规划" : "源码"}
            </small>
            <button
              className="package-detail-button"
              aria-label={`${p.name}含义`}
              aria-pressed={detailId === p.id}
              onClick={() => setDetailId(p.id)}
            >
              说明
            </button>
          </div>
        ))}
        {!filtered.length && <p className="muted">没有匹配的功能包</p>}
      </div>
      {detail && (
        <aside className="package-detail" aria-live="polite">
          <strong>{detail.name}</strong>
          <p>{detail.description}</p>
          <small className="muted">
            {detail.kind === "planning"
              ? `${detail.sourceKey} · 仅规划，未提供实现`
              : "附带源码；选择规划不会接入，需在源码包页明确发起任务"}
          </small>
        </aside>
      )}
      {!!missing.length && (
        <div className="package-missing">
          <span className="muted">目录外的已有选择</span>
          {missing.map((id) => (
            <button
              key={id}
              onClick={() =>
                setSelected(value.plannedFeatures.filter((v) => v !== id))
              }
            >
              移除 {id}
            </button>
          ))}
        </div>
      )}
      <p className="muted blueprint-footnote">
        选择仅记录规划，不安装、不调用
        AI、不付费。各项可组合，依赖与兼容性留待制作时确认。
      </p>
    </section>
  );
}
