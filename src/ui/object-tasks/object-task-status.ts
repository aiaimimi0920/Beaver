import type { ObjectTaskRecord } from "../../shared/object-tasks";

export const objectTaskStatusLabels = {
  planned: "已规划",
  queued: "已领取（待执行）",
  running: "执行中",
  awaitingAcceptance: "待验收",
  accepted: "已接受",
  failed: "失败待恢复",
  cancelled: "已撤销",
} satisfies Record<ObjectTaskRecord["status"], string>;
