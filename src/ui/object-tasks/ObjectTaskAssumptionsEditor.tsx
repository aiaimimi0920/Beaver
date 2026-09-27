import type {
  ObjectTaskAssumption,
  ObjectTaskAssumptionRecord,
  ObjectTaskPlan,
} from "../../shared/object-tasks";

let nextId = 0;

function newAssumption(): ObjectTaskAssumption {
  nextId++;
  return {
    id: `assumption-${Date.now().toString(36)}-${nextId.toString(36)}`,
    statement: "",
    basis: "",
    source: "user",
    sourceDetail: null,
  };
}

function updateAssumption(
  plan: ObjectTaskPlan,
  id: string,
  changes: Partial<ObjectTaskAssumption>,
): ObjectTaskPlan {
  return {
    ...plan,
    assumptions: plan.assumptions.map((assumption) =>
      assumption.id === id ? { ...assumption, ...changes } : assumption,
    ),
  };
}

const sourceLabels = {
  user: "用户",
  codex: "Codex",
  automatic: "自动补充",
} as const;

export function ObjectTaskAssumptionsEditor({
  plan,
  history,
  disabled,
  onPlanChange,
}: {
  plan: ObjectTaskPlan;
  history: ObjectTaskAssumptionRecord[];
  disabled: boolean;
  onPlanChange: (plan: ObjectTaskPlan) => void;
}) {
  return (
    <>
      <fieldset disabled={disabled} className="object-task-editor-fields">
        <legend>合理假设</legend>
        <p>记录暂按成立处理的事项、依据和来源，提交后会保留版本历史。</p>
        <button
          onClick={() =>
            onPlanChange({
              ...plan,
              assumptions: [...plan.assumptions, newAssumption()],
            })
          }
        >
          添加假设
        </button>
        {plan.assumptions.map((assumption) => (
          <article className="object-task-assumption" key={assumption.id}>
            <header>
              <code>{assumption.id}</code>
              <button
                aria-label={`删除假设 ${assumption.statement || assumption.id}`}
                onClick={() =>
                  onPlanChange({
                    ...plan,
                    assumptions: plan.assumptions.filter(
                      (candidate) => candidate.id !== assumption.id,
                    ),
                  })
                }
              >
                删除假设
              </button>
            </header>
            <label>
              假设内容
              <textarea
                maxLength={2_000}
                value={assumption.statement}
                onChange={(event) =>
                  onPlanChange(
                    updateAssumption(plan, assumption.id, {
                      statement: event.currentTarget.value,
                    }),
                  )
                }
              />
            </label>
            <label>
              依据
              <textarea
                maxLength={4_000}
                value={assumption.basis}
                onChange={(event) =>
                  onPlanChange(
                    updateAssumption(plan, assumption.id, {
                      basis: event.currentTarget.value,
                    }),
                  )
                }
              />
            </label>
            <label>
              来源
              <select
                value={assumption.source}
                onChange={(event) =>
                  onPlanChange(
                    updateAssumption(plan, assumption.id, {
                      source: event.currentTarget
                        .value as ObjectTaskAssumption["source"],
                    }),
                  )
                }
              >
                {Object.entries(sourceLabels).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              来源补充
              <input
                maxLength={1_000}
                value={assumption.sourceDetail ?? ""}
                onChange={(event) =>
                  onPlanChange(
                    updateAssumption(plan, assumption.id, {
                      sourceDetail: event.currentTarget.value || null,
                    }),
                  )
                }
              />
            </label>
          </article>
        ))}
      </fieldset>
      {history.length > 0 && (
        <section
          className="object-task-assumption-history"
          aria-label="已提交假设历史"
        >
          <h4>已提交假设历史</h4>
          <ol>
            {history.map((assumption) => (
              <li key={`${assumption.id}-${assumption.planRevision}`}>
                <p>
                  <strong>计划版本 {assumption.planRevision}</strong> ·{" "}
                  {sourceLabels[assumption.source]}
                  {assumption.sourceDetail && `（${assumption.sourceDetail}）`}
                </p>
                <p>{assumption.statement}</p>
                <p>依据：{assumption.basis}</p>
                <code>{assumption.id}</code>
              </li>
            ))}
          </ol>
        </section>
      )}
    </>
  );
}
