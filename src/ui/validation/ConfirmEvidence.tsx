import { useState } from "react";
import { Dialog } from "../components";
import type { Mutate, ValidationRun } from "./types";
import { perform } from "./useValidation";

export function ConfirmEvidence({
  run,
  selected,
  mutate,
  busy,
  close,
  saved,
}: {
  run: ValidationRun;
  selected?: string;
  mutate: Mutate;
  busy: boolean;
  close: () => void;
  saved: () => void;
}) {
  const [ids, setIds] = useState(
    selected ? [selected] : run.evidence.map((e) => e.id),
  );
  return (
    <Dialog
      title="确认这次运行的画面正确"
      close={close}
      className="validation-dialog"
    >
      <p>
        你正在认可运行 <code>{run.id}</code>，游戏版本{" "}
        <code>{run.snapshotId}</code>。
      </p>
      <p>
        仅确认你已查看的画面。完整认可全部必需证据后，才建立这条流程的用户基准；其他运行的结果不随之改变。
      </p>
      <div className="validation-toolbar">
        <button onClick={() => setIds(run.evidence.map((e) => e.id))}>
          选中全部画面
        </button>
        <button onClick={() => setIds([])}>取消选择</button>
      </div>
      {run.evidence.map((e) => (
        <label className="check" key={e.id}>
          <input
            type="checkbox"
            checked={ids.includes(e.id)}
            onChange={(event) =>
              setIds(
                event.target.checked
                  ? [...ids, e.id]
                  : ids.filter((id) => id !== e.id),
              )
            }
          />
          {e.kind === "video" ? "视频" : "截图"} · {e.point} ·{" "}
          {e.start.toFixed(2)}–{e.end.toFixed(2)} s
        </label>
      ))}
      <footer>
        <button onClick={close}>返回查看</button>
        <button
          className="primary"
          disabled={busy || !ids.length}
          onClick={() =>
            perform(
              mutate("validation.evidence.confirm", {
                runId: run.id,
                snapshotId: run.snapshotId,
                evidenceIds: ids,
              }).then(() => {
                saved();
                close();
              }),
            )
          }
        >
          我已查看，确认所选画面正确
        </button>
      </footer>
    </Dialog>
  );
}
