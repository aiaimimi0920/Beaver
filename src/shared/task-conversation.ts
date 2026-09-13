import type { TaskEvent } from "./types";
import { z } from "zod";

const healthSchema = z.object({
  phase: z.enum(["model", "tool", "compacting", "finishing"]),
  lastProgressAt: z.string(),
  activeTools: z.number().int().nonnegative(),
  retries: z.number().int().nonnegative(),
  pauseAfterSeconds: z.number().positive(),
});

export function executionStatus(
  events: TaskEvent[],
  now = Date.now(),
  since = 0,
): string | undefined {
  const event = events.findLast(
    (event) => event.kind === "execution" && Date.parse(event.time) >= since,
  );
  if (!event) return;
  try {
    const health = healthSchema.parse(JSON.parse(event.text));
    if (health.phase === "finishing")
      return "正在停止执行器并保存任务结果 · 尚未合入项目";
    const progressAt = Date.parse(health.lastProgressAt);
    if (!Number.isFinite(progressAt)) return;
    const seconds = Math.max(0, Math.floor((now - progressAt) / 1000));
    const retry = health.retries ? ` · 已重试 ${health.retries} 次` : "";
    if (health.phase === "tool")
      return `工具运行中（${health.activeTools} 个） · 距有效进展 ${seconds} 秒${retry} · 工具执行期间暂停模型等待计时`;
    return `${health.phase === "compacting" ? "正在压缩上下文" : "等待模型输出"} · 距有效进展 ${seconds} 秒${retry} · 无进展 ${health.pauseAfterSeconds} 秒后暂停并保留工作副本`;
  } catch {
    return;
  }
}

export function conversationText(events: TaskEvent[], report?: string): string {
  const groups: { kind: string; text: string }[] = [];
  for (const event of events) {
    if (
      ![
        "user",
        "assistant",
        "question",
        "decision",
        "providerError",
        "watchdog",
        "recovery",
        "contextRestart",
      ].includes(event.kind)
    )
      continue;
    const previous = groups.at(-1);
    if (event.kind === "assistant" && previous?.kind === "assistant")
      previous.text += event.text;
    else groups.push({ kind: event.kind, text: event.text });
  }
  const text = groups
    .map(
      (group) =>
        `${group.kind === "user" ? "你" : group.kind === "decision" ? "决策记录（逐项注明来源）" : ["assistant", "question"].includes(group.kind) ? "Codex" : "执行状态"}\n${group.text}`,
    )
    .join("\n\n");
  return report && !text.includes(report)
    ? `${text}\n\nCodex\n${report}`
    : text;
}
