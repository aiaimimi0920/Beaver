import { useState } from "react";
import type { Project } from "../shared/types";
import { call, type Run } from "./api";

interface CallRecord {
  seq: number;
  startedAt: string;
  method: string;
  source: string;
  status: string;
  durationMs?: number | null;
  error?: string;
}

export function NprPackagePanel({
  project,
  run,
}: {
  project: Project;
  run: Run;
}) {
  const [engine, setEngine] = useState(
    project.npr?.runtime?.godot ?? project.npr?.godot ?? "",
  );
  const [busy, setBusy] = useState(false);
  const [records, setRecords] = useState<CallRecord[]>([]);
  const [cursor, setCursor] = useState(0);
  const ready = project.npr?.status === "ready";
  return (
    <section className="inline-help">
      <h3>
        NPR 人物模块 <small>1.0.0</small>
      </h3>
      <p>
        {ready
          ? "已安装：Codex 可发现人物制作、校验和预览工作流。"
          : project.npr?.status === "failed"
            ? "安装失败，项目已保留。可更正引擎路径后重试。"
            : "启用后安装人物插件，并向 Codex 提供模型规范与标准工作流。"}
      </p>
      {project.npr?.error && <p role="alert">{project.npr.error}</p>}
      <label className="field">
        <span>定制 Godot 引擎路径</span>
        <input
          aria-label="NPR 模块引擎路径"
          value={engine}
          disabled={busy || ready}
          onChange={(event) => setEngine(event.target.value)}
        />
      </label>
      {!ready && (
        <button
          disabled={busy || !engine.trim()}
          onClick={() => {
            setBusy(true);
            void run(async () => {
              await call("project.npr.install", {
                id: project.id,
                godot: engine.trim(),
              });
            }).finally(() => setBusy(false));
          }}
        >
          {busy ? "安装中…" : "安装 / 重试 NPR 模块"}
        </button>
      )}
      {ready && (
        <p className="muted">
          引擎：{project.npr?.runtime?.engineVersion}
          。安装状态与具体角色的美术、渲染验收分别记录。
        </p>
      )}
      <details>
        <summary>项目调用日志</summary>
        <p className="muted">
          记录 API、MCP、界面和 Codex
          工具调用的状态与耗时；参数仅保留摘要，错误会脱敏。完整分页查询可使用
          logs.query。
        </p>
        <button
          onClick={() =>
            void run(async () => {
              const page = await call<{
                records: CallRecord[];
                nextAfter: number;
              }>("logs.query", { projectId: project.id, limit: 100 });
              setRecords(page.records);
              setCursor(page.nextAfter);
            })
          }
        >
          读取 / 刷新日志
        </button>
        <button
          disabled={!cursor}
          onClick={() =>
            void run(async () => {
              const page = await call<{
                records: CallRecord[];
                nextAfter: number;
              }>("logs.query", {
                projectId: project.id,
                after: cursor,
                limit: 100,
              });
              setRecords(page.records);
              setCursor(page.nextAfter);
            })
          }
        >
          读取后续记录
        </button>
        <pre
          style={{ maxHeight: 280, overflow: "auto", whiteSpace: "pre-wrap" }}
        >
          {records
            .map(
              (record) =>
                `${record.startedAt} [${record.source}] ${record.method}: ${record.status} (${record.durationMs ?? "—"} ms)${record.error ? "\n" + record.error : ""}`,
            )
            .join("\n") || "尚未读取日志。"}
        </pre>
      </details>
    </section>
  );
}
