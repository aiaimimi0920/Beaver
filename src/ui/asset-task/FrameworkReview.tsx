import { useState } from "react";
import type { DeliveryCandidate } from "../../shared/asset-delivery";
import type { FrameworkAction, FrameworkJob } from "../../shared/framework";

export function FrameworkReview({
  candidate,
  revision,
  disabled,
  action,
  start,
}: {
  candidate: DeliveryCandidate;
  revision: number;
  disabled: boolean;
  action: FrameworkAction;
  start: (job: FrameworkJob) => Promise<void>;
}) {
  const [note, setNote] = useState("");
  const [paths, setPaths] = useState<string[]>([]);
  return (
    <section>
      <h4>检查当前候选</h4>
      <button
        disabled={disabled}
        onClick={() => void start({ kind: "check", candidateId: candidate.id })}
      >
        运行已配置的技术检查
      </button>
      <p>
        视觉评价使用当前候选的冻结 PNG。请先查看上方文件预览，再选择评价依据。
      </p>
      {Object.keys(candidate.files)
        .filter((p) => p.toLowerCase().endsWith(".png"))
        .map((path) => (
          <label key={path}>
            <input
              type="checkbox"
              checked={paths.includes(path)}
              disabled={disabled}
              onChange={(e) =>
                setPaths(
                  e.target.checked
                    ? [...paths, path]
                    : paths.filter((p) => p !== path),
                )
              }
            />
            {path}
          </label>
        ))}
      <label>
        视觉评价说明
        <textarea
          value={note}
          disabled={disabled}
          maxLength={4000}
          onChange={(e) => setNote(e.target.value)}
        />
      </label>
      <div className="asset-toolbar">
        {(["pass", "fail"] as const).map((verdict) => (
          <button
            key={verdict}
            disabled={
              disabled ||
              !note.trim() ||
              paths.length === 0 ||
              paths.length > 16
            }
            onClick={() =>
              void action({
                operation: "judge",
                candidateId: candidate.id,
                configurationRevision: revision,
                verdict,
                note,
                paths,
              })
            }
          >
            {verdict === "pass" ? "记录视觉通过" : "记录视觉未通过"}
          </button>
        ))}
      </div>
      <p>
        此处保存用户视觉评价；阶段批准仍由“批准并继续”执行，并核验必需技术检查。
      </p>
    </section>
  );
}
