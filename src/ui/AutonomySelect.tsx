import {
  autonomyLevels,
  autonomyLabel,
  type AskRatio,
} from "../shared/autonomy";

export function AutonomySelect({
  value,
  change,
  inherit = false,
  disabled = false,
  label = "自动决策",
}: {
  value?: AskRatio | null;
  change: (value: AskRatio | null) => void;
  inherit?: boolean;
  disabled?: boolean;
  label?: string;
}) {
  return (
    <select
      aria-label={label}
      disabled={disabled}
      value={value ?? (inherit ? "inherit" : 100)}
      onChange={(e) =>
        change(
          e.target.value === "inherit"
            ? null
            : (Number(e.target.value) as AskRatio),
        )
      }
    >
      {inherit && <option value="inherit">跟随全局</option>}
      {autonomyLevels.map((r) => (
        <option key={r} value={r}>
          {autonomyLabel(r)}
        </option>
      ))}
    </select>
  );
}
