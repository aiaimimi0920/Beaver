import { useEffect, useMemo, useSyncExternalStore } from "react";
import type { ObjectTaskWorkspace } from "./object-task-workspace";
import { ObjectTaskTitleController } from "./object-task-title-suggestion";

export function ObjectTaskTitleSuggestion({
  workspace,
  taskId,
  disabled,
}: {
  workspace: ObjectTaskWorkspace;
  taskId: string;
  disabled: boolean;
}) {
  const controller = useMemo(
    () => new ObjectTaskTitleController(workspace, taskId),
    [workspace, taskId],
  );
  const state = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
    controller.getSnapshot,
  );
  useEffect(() => () => controller.dismiss(), [controller]);
  return (
    <div aria-label="Codex 标题建议">
      <button
        disabled={disabled || state.pending}
        onClick={() => void controller.request()}
      >
        {state.pending ? "正在请求 Codex…" : "Codex 建议标题"}
      </button>
      {state.pending && (
        <span role="status">正在生成候选标题，原内容保持不变。</span>
      )}
      {state.candidate && (
        <div>
          <p>候选标题：{state.candidate}</p>
          <button disabled={disabled} onClick={controller.accept}>
            采用标题
          </button>
          <button onClick={controller.dismiss}>放弃建议</button>
        </div>
      )}
      {state.error && <p role="alert">{state.error}</p>}
    </div>
  );
}
