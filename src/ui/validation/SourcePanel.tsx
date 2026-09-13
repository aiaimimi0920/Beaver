import { useState } from "react";
import { Field } from "../components";
import type { SourceReference } from "./flow";
import type { ValidationRun } from "./types";
import { useValidationQuery } from "./useValidation";

interface Source {
  path: string;
  snapshotId: string;
  sha256: string;
  historical: string;
  current: string | null;
}
export function SourcePanel({
  run,
  references,
}: {
  run: ValidationRun;
  references: SourceReference[];
}) {
  const [path, setPath] = useState("");
  const { data, error, refresh } = useValidationQuery<Source>(
    "validation.source",
    path ? { projectId: run.projectId, runId: run.id, path } : null,
    () => false,
  );
  const sources = { author: "流程作者", runtime: "运行时报告", ai: "AI 推断" };
  return (
    <details className="validation-source" open={!!path}>
      <summary>相关代码、场景与资源</summary>
      {references.length ? (
        <ul>
          {references.map((ref, i) => (
            <li key={`${ref.path}:${i}`}>
              <button onClick={() => setPath(ref.path)}>{ref.path}</button>
              <small>
                {sources[ref.source]} {ref.node && ` · 节点 ${ref.node}`}{" "}
                {ref.symbol && ` · 符号 ${ref.symbol}`}
              </small>
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">此采集点没有精确引用，可查看运行快照中的文件。</p>
      )}
      <Field label="快照文件">
        <select
          value={path.replace(/^res:\/\//, "")}
          onChange={(e) => setPath(e.target.value)}
        >
          <option value="">选择历史文件</option>
          {run.sourcePaths?.map((p) => (
            <option key={p}>{p}</option>
          ))}
        </select>
      </Field>
      {path && <button onClick={refresh}>刷新当前文件对照</button>}
      {error && <p className="validation-error">{error}</p>}
      {data && (
        <>
          <p className="muted">
            {data.path} · 历史快照 {data.snapshotId.slice(0, 10)} ·{" "}
            {data.current === data.historical
              ? "当前内容相同"
              : "当前内容已变化或不可读取"}
          </p>
          <div className="validation-source-grid">
            <section>
              <h4>画面对应的历史代码</h4>
              <pre>{data.historical}</pre>
            </section>
            <section>
              <h4>当前代码</h4>
              <pre>{data.current ?? "当前文件不存在、过大或非 UTF-8 文本"}</pre>
            </section>
          </div>
        </>
      )}
    </details>
  );
}
