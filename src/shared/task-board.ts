import { z } from "zod";
import type { Task } from "./types";

export const directionSchema = z.enum([
  "general",
  "story",
  "gameplay",
  "visual",
  "audio",
  "engineering",
  "review",
  "release",
]);
export type TaskDirection = z.infer<typeof directionSchema>;
export const directions = {
  general: { label: "综合", icon: "compass" },
  story: { label: "剧情与设定", icon: "book" },
  gameplay: { label: "玩法与系统", icon: "puzzle" },
  visual: { label: "美术与场景", icon: "palette" },
  audio: { label: "音乐与声音", icon: "music" },
  engineering: { label: "程序与技术", icon: "code" },
  review: { label: "测试与审查", icon: "review" },
  release: { label: "构建与发行", icon: "play" },
} as const;
export const boardColumns = [
  { id: "queued", label: "待开始" },
  { id: "running", label: "执行中" },
  { id: "attention", label: "需要你" },
  { id: "review", label: "待验收" },
  { id: "done", label: "已完成" },
] as const;
export function taskColumn(task: Task): (typeof boardColumns)[number]["id"] {
  if (task.status === "waitingChildren") return "running";
  if (task.status === "queued" || task.status === "running") return task.status;
  if (task.status === "completed") return task.accepted ? "done" : "review";
  if (task.status === "rolledBack") return "done";
  return "attention";
}
export function taskDirection(task: Task): TaskDirection {
  return (
    task.direction ?? (task.capability === "review" ? "review" : "general")
  );
}

export function taskAttention(task: Task): string {
  if (task.clarifications?.some((item) => !item.answers)) return "待回答";
  if (task.status === "awaitingInput") return "待回答";
  if (task.status === "conflict") return "待整合";
  if (task.status === "failed") return "待处理";
  if (task.status === "interrupted") return "待继续";
  if (task.status === "completed" && !task.accepted) return "待验收";
  return "";
}
