import type { ObjectTaskDefinition } from "../../shared/object-task-revisions";
import { objectTaskRequirementLabels } from "./ObjectTaskRequirementField";

export function ObjectTaskDefinitionView({
  label,
  definition,
}: {
  label: string;
  definition: ObjectTaskDefinition;
}) {
  return (
    <section className="object-task-definition-view" aria-label={label}>
      <h4>{label}</h4>
      <dl>
        <dt>工作范围</dt>
        <dd>{objectTaskRequirementLabels[definition.requirement]}</dd>
        <dt>标题</dt>
        <dd>{definition.title}</dd>
        <dt>任务目标</dt>
        <dd>{definition.prompt}</dd>
        <dt>验收要求</dt>
        <dd>{definition.acceptance || "未填写"}</dd>
        <dt>尚待规划</dt>
        <dd>
          {definition.pendingPlanning || "无已登记事项（不代表目标完成）"}
        </dd>
        <dt>显式依赖</dt>
        <dd>
          {definition.dependsOn.length ? (
            <ul>
              {definition.dependsOn.map((id) => (
                <li key={id}>
                  <code>{id}</code>
                </li>
              ))}
            </ul>
          ) : (
            "无显式依赖"
          )}
        </dd>
      </dl>
    </section>
  );
}

export function ObjectTaskDefinitionComparison({
  before,
  after,
}: {
  before: ObjectTaskDefinition;
  after: ObjectTaskDefinition;
}) {
  return (
    <div
      className="object-task-definition-comparison"
      aria-label="任务定义修订对比"
    >
      <ObjectTaskDefinitionView label="修订前" definition={before} />
      <ObjectTaskDefinitionView label="修订后" definition={after} />
    </div>
  );
}
