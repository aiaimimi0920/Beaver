import type { ObjectTaskProposal } from "../../shared/object-tasks";

export const objectTaskRequirementLabels = {
  required: "必要工作",
  optional: "可选工作",
};

export function ObjectTaskRequirementField({
  value,
  onChange,
}: {
  value: ObjectTaskProposal["requirement"];
  onChange: (value: ObjectTaskProposal["requirement"]) => void;
}) {
  return (
    <label>
      工作范围
      <select
        value={value}
        onChange={(event) =>
          onChange(
            event.currentTarget.value === "optional" ? "optional" : "required",
          )
        }
      >
        <option value="required">必要工作</option>
        <option value="optional">可选工作</option>
      </select>
      <small>
        可选工作仍须满足显式依赖、阶段检查和发布验收。取消必要工作不会完成父任务。
      </small>
    </label>
  );
}
