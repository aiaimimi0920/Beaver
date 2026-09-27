import type { ObjectTaskRecord } from "../../shared/object-tasks";

// Count responsibility descendants only; dependencies do not belong to progress.
export function objectTaskProgress(
  task: ObjectTaskRecord,
  children: ReadonlyMap<string, readonly ObjectTaskRecord[]>,
): string[] {
  if (task.granularity === "fine") return [];
  const fine: ObjectTaskRecord[] = [];
  const requiredFine: ObjectTaskRecord[] = [];
  const requiredMedium: ObjectTaskRecord[] = [];
  const unplanned: ObjectTaskRecord[] = [];
  const pending: ObjectTaskRecord[] = [];
  const visited = new Set<string>();
  function visit(current: ObjectTaskRecord, required = true) {
    if (visited.has(current.id)) return;
    visited.add(current.id);
    if (current.pendingPlanning?.trim()) pending.push(current);
    if (current.granularity === "fine") {
      fine.push(current);
      if (required) requiredFine.push(current);
      return;
    }
    const expected = current.granularity === "coarse" ? "medium" : "fine";
    const items = (children.get(current.id) ?? []).filter(
      (child) => child.granularity === expected,
    );
    if (!items.length) unplanned.push(current);
    for (const child of items) {
      const childRequired = required && child.requirement !== "optional";
      if (child.granularity === "medium" && childRequired)
        requiredMedium.push(child);
      visit(child, childRequired);
    }
  }
  visit(task);
  const count = (status: ObjectTaskRecord["status"]) =>
    fine.filter((item) => item.status === status).length;
  const result = fine.length
    ? [
        `已接受精修 ${count("accepted")} / 已规划精修 ${fine.length}（当前计划）`,
        `等待开始 ${count("planned") + count("queued")} · 执行中 ${count("running")} · 待验收 ${count("awaitingAcceptance")} · 失败 ${count("failed")} · 已撤销 ${count("cancelled")}`,
      ]
    : [
        task.granularity === "medium"
          ? "尚未规划精修，进度未知。"
          : "尚无已规划精修，进度未知。",
      ];
  if (unplanned.length) {
    result.push(
      `尚未细化子任务（${unplanned.length}）：${unplanned.map((item) => item.title).join("、")}。整体进度未知。`,
    );
  }
  for (const item of pending) {
    result.push(
      `尚待规划（${item.title}）：${item.pendingPlanning}。整体进度未知。`,
    );
  }
  const accepted = (items: ObjectTaskRecord[]) =>
    items.filter((item) => item.status === "accepted").length;
  const cancelled = (items: ObjectTaskRecord[]) =>
    items.filter((item) => item.status === "cancelled").length;
  result.push(
    `必要精修已接受 ${accepted(requiredFine)} / ${requiredFine.length} · 可选范围精修 ${fine.length - requiredFine.length}`,
  );
  if (task.granularity === "coarse") {
    result.push(
      `必要中修已接受 ${accepted(requiredMedium)} / ${requiredMedium.length}（包含对象发布要求）`,
    );
  }
  if (cancelled(requiredFine) || cancelled(requiredMedium)) {
    result.push(
      "存在已撤销的必要工作，父任务不能据此完成；须修订目标与验收并记录决定。",
    );
  }
  if (!requiredFine.length)
    result.push("当前无已规划必要精修，不代表目标已经完成。");
  result.push("以上仅为当前计划计数；撤销不算接受，整体完成以父任务验收为准。");
  return result;
}
