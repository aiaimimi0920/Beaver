import type { ObjectBaseline } from "../../shared/object-framework";

export function ObjectTaskBaselineEditor({
  baseline,
  onChange,
}: {
  baseline: ObjectBaseline | undefined;
  onChange: (baseline: ObjectBaseline) => void;
}) {
  const policy = baseline?.basePolicy ?? "latestAccepted";
  return (
    <div>
      <label>
        工作基准
        <select
          value={policy}
          onChange={(event) => {
            const value = event.currentTarget.value;
            if (value === "pinnedVersion") {
              onChange({ basePolicy: value, selectedVersionId: "" });
            } else if (value === "latestAccepted" || value === "empty") {
              onChange({ basePolicy: value });
            }
          }}
        >
          <option value="latestAccepted">最新已接受版本</option>
          <option value="pinnedVersion">指定历史版本</option>
          <option value="empty">空基准</option>
        </select>
      </label>
      {baseline?.basePolicy === "pinnedVersion" && (
        <label>
          已接受版本 ID
          <input
            value={baseline.selectedVersionId}
            onChange={(event) =>
              onChange({
                basePolicy: "pinnedVersion",
                selectedVersionId: event.currentTarget.value,
              })
            }
          />
        </label>
      )}
      <p>
        {policy === "latestAccepted"
          ? "领取任务时固定最新已接受版本。对象尚无已接受版本时，请明确选择空基准。"
          : policy === "pinnedVersion"
            ? "从此对象的指定已接受版本开始；后续接受的新版本不会替换该基准。"
            : "从空工作目录开始；已有对象的接受版本仍保留。"}
      </p>
    </div>
  );
}
