import type { DemoTask } from "./mock-tasks";

export function IterationProgress({ task }: { task: DemoTask }) {
  const delivered =
    task.status === "已验收" ||
    (task.status === "待验收" && task.progress === 100);
  const progress = delivered ? 100 : task.progress;
  return (
    <span
      className={`op-iteration-progress${delivered ? " is-done" : ""}`}
      role={delivered ? "img" : "progressbar"}
      aria-label={`${task.title}：${delivered ? "已交付" : "制作进度"}`}
      aria-valuemin={delivered ? undefined : 0}
      aria-valuemax={delivered ? undefined : 100}
      aria-valuenow={delivered ? undefined : progress}
    >
      <svg viewBox="0 0 36 36" aria-hidden="true">
        <circle className="op-iteration-track" cx="18" cy="18" r="15" />
        <circle
          className="op-iteration-fill"
          cx="18"
          cy="18"
          r="15"
          pathLength="100"
          strokeDasharray={`${progress} 100`}
          transform="rotate(-90 18 18)"
        />
      </svg>
      <span aria-hidden="true">{delivered ? "✓" : `${progress}%`}</span>
    </span>
  );
}
