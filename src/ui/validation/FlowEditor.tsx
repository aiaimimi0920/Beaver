import { useState } from "react";
import type { Task } from "../../shared/types";
import { Dialog, Field } from "../components";
import { errorMessage } from "../notification-state";
import { categories, newDefinition, type Category, type Flow } from "./flow";
import type { Mutate } from "./types";
import { FlowConfig } from "./FlowConfig";
import { StepsEditor } from "./StepsEditor";

export function FlowEditor({
  flow,
  taskId,
  tasks,
  mutate,
  close,
  saved,
}: {
  flow?: Flow;
  taskId: string;
  tasks: Task[];
  mutate: Mutate;
  close: () => void;
  saved: (flow: Flow) => void;
}) {
  const [definition, setDefinition] = useState(
    () => flow?.definition ?? newDefinition(taskId),
  );
  const [reason, setReason] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [json, setJson] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function save() {
    setBusy(true);
    setError("");
    try {
      const next = await mutate<Flow>("validation.flow.save", {
        definition: advanced ? (JSON.parse(json) as unknown) : definition,
        expectedRevision: flow?.revision ?? 0,
        reason,
      });
      saved(next);
      close();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog
      title={flow ? `编辑流程 · v${flow.revision}` : "新增画面流程"}
      close={close}
      className="validation-flow-dialog"
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <label className="check">
          <input
            type="checkbox"
            checked={advanced}
            onChange={(e) => {
              setAdvanced(e.target.checked);
              setJson(JSON.stringify(definition, null, 2));
            }}
          />
          高级 JSON 编辑（包含步骤引用、遮罩与漫游准备步骤）
        </label>
        {advanced ? (
          <>
            <p className="muted">
              保存时由后端完整校验。返回表单会丢弃尚未保存的 JSON 修改。
            </p>
            <textarea
              className="validation-json"
              aria-label="完整流程 JSON"
              value={json}
              onChange={(e) => setJson(e.target.value)}
              spellCheck={false}
            />
          </>
        ) : (
          <>
            <div className="validation-form-grid">
              <Field label="名称">
                <input
                  required
                  value={definition.name}
                  onChange={(e) =>
                    setDefinition({ ...definition, name: e.target.value })
                  }
                />
              </Field>
              <Field label="稳定标识">
                <input
                  required
                  pattern="[A-Za-z0-9_-]{1,100}"
                  disabled={!!flow}
                  value={definition.key}
                  onChange={(e) =>
                    setDefinition({ ...definition, key: e.target.value })
                  }
                />
              </Field>
              <Field label="分类">
                <select
                  value={definition.category}
                  onChange={(e) =>
                    setDefinition({
                      ...definition,
                      category: e.target.value as Category,
                    })
                  }
                >
                  {Object.entries(categories).map(([id, name]) => (
                    <option key={id} value={id}>
                      {name}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="入口场景（留空使用主场景）">
                <input
                  value={definition.entry}
                  placeholder="res://scenes/main.tscn"
                  onChange={(e) =>
                    setDefinition({ ...definition, entry: e.target.value })
                  }
                />
              </Field>
            </div>
            <Field label="流程目的与观察点">
              <textarea
                required
                value={definition.purpose}
                onChange={(e) =>
                  setDefinition({ ...definition, purpose: e.target.value })
                }
              />
            </Field>
            <Field label="关联任务（可多选）">
              <select
                multiple
                value={definition.taskIds}
                onChange={(e) =>
                  setDefinition({
                    ...definition,
                    taskIds: Array.from(
                      e.target.selectedOptions,
                      (o) => o.value,
                    ),
                  })
                }
              >
                {tasks.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.title}
                  </option>
                ))}
              </select>
            </Field>
            <FlowConfig definition={definition} change={setDefinition} />
            {definition.roaming && !definition.steps.length ? (
              <p className="validation-notice">
                保存时生成并固化路线；运行后可查看实际步骤。
              </p>
            ) : (
              <StepsEditor
                steps={definition.steps}
                change={(steps) => setDefinition({ ...definition, steps })}
              />
            )}
            <Field label="相关代码或资源（每行一项，更多定位可用高级 JSON）">
              <textarea
                value={definition.references.map((r) => r.path).join("\n")}
                onChange={(e) =>
                  setDefinition({
                    ...definition,
                    references: e.target.value
                      .split("\n")
                      .filter(Boolean)
                      .map(
                        (path) =>
                          definition.references.find(
                            (r) => r.path === path,
                          ) ?? { path, node: "", symbol: "", source: "author" },
                      ),
                  })
                }
              />
            </Field>
            <Field label="退役原因（填写后退出后续诊断范围）">
              <input
                value={definition.retiredReason}
                onChange={(e) =>
                  setDefinition({
                    ...definition,
                    retiredReason: e.target.value,
                  })
                }
                placeholder="仅在玩法已删除或明确不再适用时填写"
              />
            </Field>
          </>
        )}
        <Field label="本次修订原因">
          <input
            required={!!flow}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="保留流程修改和退役依据"
          />
        </Field>
        {error && (
          <p className="validation-error" role="alert">
            {error}
          </p>
        )}
        <footer>
          <button type="button" onClick={close}>
            取消
          </button>
          <button className="primary" disabled={busy}>
            {busy ? "保存中…" : "保存流程"}
          </button>
        </footer>
      </form>
    </Dialog>
  );
}
