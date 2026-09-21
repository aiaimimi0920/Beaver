import { useState } from "react";
import type { DemoObject } from "./mock-objects";
import { stageNames } from "./mock-tasks";
import { ManufactureStageWorkspace } from "./ManufactureStageWorkspace";

export function CreationView({
  object,
  initialStage,
  initialVersion,
  taskTitle,
}: {
  object: DemoObject;
  initialStage?: number;
  initialVersion?: string;
  taskTitle?: string;
}) {
  const idle = !object.taskId || object.status === "已验收";
  const currentStage = idle ? 5 : object.status === "等待依赖" ? 0 : 2;
  const [stage, setStage] = useState(initialStage ?? currentStage);
  const [baseVersion, setBaseVersion] = useState(
    initialVersion ?? object.version,
  );
  const [stageUpdates, setStageUpdates] = useState<
    Record<string, Record<number, string>>
  >({});
  const updates = stageUpdates[baseVersion] ?? {};
  const pending =
    !idle && (stage > currentStage || object.status === "等待依赖");
  const titles =
    object.kind === "character"
      ? stageNames
      : [
          "目标与规格",
          "结构与组成",
          "内容制作",
          "行为与联动",
          "工程整合",
          "对象验收",
        ];
  const versions = [
    ...new Set(
      [
        object.version,
        object.accepted,
        ...(object.id === "O-001" ? ["r02"] : []),
      ].filter((value): value is string => !!value),
    ),
  ];
  return (
    <section className="op-page op-creation-page">
      <div className="op-creation-layout">
        <ManufactureStageWorkspace
          object={object}
          stage={stage}
          title={titles[stage]!}
          version={baseVersion}
          taskTitle={taskTitle}
          pending={pending}
          stale={updates[stage]}
          regenerated={(scope) =>
            setStageUpdates((previous) => ({
              ...previous,
              [baseVersion]: Object.fromEntries(
                titles.map((_, index) => [
                  index,
                  index < stage
                    ? (previous[baseVersion]?.[index] ?? "")
                    : index === stage
                      ? "演示新轮次"
                      : scope === "stage"
                        ? "待更新"
                        : "等待重新生成",
                ]),
              ),
            }))
          }
        />
        <aside className="op-stage-rail">
          <p className="op-section-label">任务对象</p>
          <div className="op-creation-object" title={object.name}>
            {object.name}
          </div>
          <label className="op-section-label" htmlFor="op-creation-version">
            制作基准版本
          </label>
          <select
            id="op-creation-version"
            value={baseVersion}
            onChange={(event) => setBaseVersion(event.target.value)}
          >
            {versions.map((value) => (
              <option value={value} key={value}>
                {value} ·{" "}
                {value === object.version
                  ? "最新版本"
                  : value === object.accepted
                    ? "已验收版本"
                    : "历史备份"}
              </option>
            ))}
          </select>
          <p className="op-section-label">线性制作阶段</p>
          <nav className="op-stage-list" aria-label="制作阶段">
            {titles.map((title, index) => {
              const done = idle || index < currentStage;
              const active =
                !idle && index === currentStage && object.status !== "等待依赖";
              return (
                <button
                  key={title}
                  className={stage === index ? "is-selected" : ""}
                  aria-current={stage === index ? "step" : undefined}
                  onClick={() => setStage(index)}
                >
                  <span
                    className={
                      done && !updates[index]
                        ? "op-step-number is-done"
                        : "op-step-number"
                    }
                  >
                    {done && !updates[index]
                      ? "✓"
                      : String(index + 1).padStart(2, "0")}
                  </span>
                  <span>
                    <strong>{title}</strong>
                    <small>
                      {updates[index] ||
                        (done
                          ? "已交付"
                          : active
                            ? object.status
                            : "等待上游验收")}
                    </small>
                  </span>
                </button>
              );
            })}
          </nav>
        </aside>
      </div>
    </section>
  );
}
