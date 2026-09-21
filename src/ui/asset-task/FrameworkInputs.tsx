import { useState } from "react";
import type { AssetWork } from "../../shared/asset-work";
import type { FrameworkCommand, FrameworkJob } from "../../shared/framework";

export function FrameworkInputs({
  work,
  disabled,
  start,
}: {
  work?: AssetWork;
  disabled: boolean;
  start: (job: FrameworkJob) => Promise<void>;
}) {
  const [attempt, setAttempt] = useState("");
  const [command, setCommand] = useState(
    '{"executable":"","sha256":"","args":[],"timeoutSeconds":120,"files":{}}',
  );
  const [paths, setPaths] = useState("");
  const [error, setError] = useState("");
  const attempts = work?.attempts.filter((a) => a.inputFiles?.length) ?? [];
  const selected =
    attempts.find((a) => a.id === attempt)?.id ?? attempts[0]?.id;
  return (
    <details>
      <summary>输入导出与依赖发现</summary>
      <label>
        冻结输入所属尝试
        <select
          value={selected ?? ""}
          disabled={disabled}
          onChange={(e) => setAttempt(e.target.value)}
        >
          {attempts.map((a) => (
            <option key={a.id} value={a.id}>
              {a.definition.title} · {a.id}
            </option>
          ))}
        </select>
      </label>
      <button
        disabled={disabled || !selected}
        onClick={() => {
          if (selected)
            void start({ kind: "inputExport", attemptId: selected });
        }}
      >
        导出此尝试的冻结输入
      </button>
      <p>导出目录出现在操作结果中；同一路径的不同历史版本分别导出。</p>
      <label>
        Blender 命令 JSON
        <textarea
          value={command}
          disabled={disabled}
          rows={4}
          onChange={(e) => setCommand(e.target.value)}
        />
      </label>
      <label>
        工作副本内 .blend 相对路径，每行一个
        <textarea
          value={paths}
          disabled={disabled}
          onChange={(e) => setPaths(e.target.value)}
        />
      </label>
      {error && <p role="alert">{error}</p>}
      <button
        disabled={disabled || !paths.trim()}
        onClick={() => {
          try {
            const parsed = JSON.parse(command) as FrameworkCommand;
            setError("");
            void start({
              kind: "dependencies",
              command: parsed,
              paths: paths
                .split(/\r?\n/)
                .map((p) => p.trim())
                .filter(Boolean),
            });
          } catch {
            setError("Blender 命令必须是有效 JSON。");
          }
        }}
      >
        扫描已保存的外部依赖
      </button>
      <p>
        扫描不会修改模型。外部、缺失和未覆盖依赖须明确处理；发现结果不会自动替代
        work.begin 的输入声明。
      </p>
    </details>
  );
}
