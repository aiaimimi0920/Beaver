import { useSyncExternalStore } from "react";
import type { ObjectAttemptTrace } from "./object-attempt-trace";

const operations = {
  commandExecution: "命令执行",
  fileChange: "文件修改",
  mcpToolCall: "MCP 工具",
  dynamicToolCall: "动态工具",
  webSearch: "网页搜索",
  turn: "模型回合",
  provider: "模型服务",
};
const phases = { started: "开始", completed: "结束", error: "错误" };
const statuses = {
  completed: "完成",
  failed: "失败",
  declined: "拒绝",
  inProgress: "进行中",
  unknown: "未提供结果",
  interrupted: "中断",
  retrying: "等待重试",
};

export function ObjectAttemptTracePanel({
  session,
}: {
  session: ObjectAttemptTrace;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  return (
    <details>
      <summary>操作轨迹</summary>
      <p>
        记录收到的操作通知及结果，不包含命令正文、模型文本或日志。通知结束不代表成果已验收；未收到结束通知的操作结果未知。
      </p>
      <button
        disabled={state.loading}
        onClick={() => {
          void session.refresh();
        }}
      >
        读取或刷新操作轨迹
      </button>
      {state.loading && <p role="status">正在读取操作轨迹…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.response?.entries.length === 0 && (
        <p>没有采集到操作轨迹。旧版本执行不会补录。</p>
      )}
      {state.response?.truncated && (
        <p role="status">仅保留最先收到的 256 条通知，后续通知未保存。</p>
      )}
      <ol aria-label="持久化操作轨迹">
        {state.response?.entries.map((entry) => (
          <li key={entry.sequence}>
            {entry.sequence}. {operations[entry.operation]} ·{" "}
            {phases[entry.phase]} · {statuses[entry.status]}
          </li>
        ))}
      </ol>
    </details>
  );
}
