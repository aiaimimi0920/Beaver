import { useState } from "react";
import type { Task } from "../shared/types";
import { call, type Run } from "./api";

export function SupplementWithdrawal({ task, run }: { task: Task; run: Run }) {
  const [reason, setReason] = useState("");
  if (
    !("__TAURI__" in window) ||
    !task.parentTaskId ||
    task.title !== "落实集成验收期间的补充要求" ||
    !["failed", "interrupted"].includes(task.status)
  )
    return null;
  if (task.supplementWithdrawal)
    return <span>已撤销补充步骤：{task.supplementWithdrawal.reason}</span>;
  return (
    <label>
      撤销未执行的补充步骤（保留历史，重新执行集成验收）
      <input
        aria-label="撤销补充步骤的理由"
        value={reason}
        onChange={(event) => setReason(event.target.value)}
        maxLength={2000}
      />
      <button
        disabled={!reason.trim()}
        onClick={() =>
          void run(() =>
            call("task.withdrawSupplement", {
              id: task.id,
              parentId: task.parentTaskId,
              reason: reason.trim(),
            }),
          )
        }
      >
        撤销补充步骤
      </button>
    </label>
  );
}
