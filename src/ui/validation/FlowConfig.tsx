import { Field } from "../components";
import type { Definition } from "./flow";

export function FlowConfig({
  definition,
  change,
}: {
  definition: Definition;
  change: (value: Definition) => void;
}) {
  const config = definition.config;
  const update = (values: Partial<Definition["config"]>) =>
    change({ ...definition, config: { ...config, ...values } });
  const roaming = definition.roaming;
  return (
    <>
      <fieldset>
        <legend>运行配置</legend>
        <div className="validation-form-grid">
          <Field label="宽度">
            <input
              type="number"
              min={64}
              max={3840}
              required
              value={config.width}
              onChange={(e) => update({ width: Number(e.target.value) })}
            />
          </Field>
          <Field label="高度">
            <input
              type="number"
              min={64}
              max={2160}
              required
              value={config.height}
              onChange={(e) => update({ height: Number(e.target.value) })}
            />
          </Field>
          <Field label="帧率">
            <input
              type="number"
              min={1}
              max={60}
              required
              value={config.fps}
              onChange={(e) => update({ fps: Number(e.target.value) })}
            />
          </Field>
          <Field label="随机种子">
            <input
              type="number"
              min={0}
              max={4294967295}
              required
              value={config.seed}
              onChange={(e) => update({ seed: Number(e.target.value) })}
            />
          </Field>
          <Field label="语言">
            <input
              required
              value={config.locale}
              onChange={(e) => update({ locale: e.target.value })}
            />
          </Field>
          <Field label="相似度阈值">
            <input
              type="number"
              min={0.8}
              max={1}
              step={0.001}
              required
              value={config.threshold}
              onChange={(e) => update({ threshold: Number(e.target.value) })}
            />
          </Field>
        </div>
        <Field label="初始存档（项目相对路径，每行一项）">
          <textarea
            value={config.saves.join("\n")}
            onChange={(e) =>
              update({ saves: e.target.value.split("\n").filter(Boolean) })
            }
          />
        </Field>
        <label className="check">
          <input
            type="checkbox"
            checked={definition.video}
            onChange={(e) => change({ ...definition, video: e.target.checked })}
          />
          录制真实过程视频（需要 FFmpeg）
        </label>
      </fieldset>
      {definition.category === "roaming" && (
        <fieldset>
          <legend>固定漫游路线</legend>
          <p className="muted">
            保存时将随机动作固化。后续重跑使用相同路线；探索新路线会保留原流程。
          </p>
          {!roaming && (
            <button
              type="button"
              onClick={() =>
                change({
                  ...definition,
                  roaming: {
                    actions: ["ui_left", "ui_right"],
                    rounds: 5,
                    minFrames: 15,
                    maxFrames: 60,
                    setup: [],
                  },
                  steps: [],
                })
              }
            >
              配置随机漫游
            </button>
          )}
          {roaming && (
            <>
              <Field label="可用游戏动作（每行一项）">
                <textarea
                  required
                  value={roaming.actions.join("\n")}
                  onChange={(e) =>
                    change({
                      ...definition,
                      roaming: {
                        ...roaming,
                        actions: e.target.value.split("\n").filter(Boolean),
                      },
                    })
                  }
                />
              </Field>
              <div className="validation-form-grid">
                {(
                  [
                    ["rounds", "动作轮数"],
                    ["minFrames", "最短持续帧"],
                    ["maxFrames", "最长持续帧"],
                  ] as const
                ).map(([key, title]) => (
                  <Field key={key} label={title}>
                    <input
                      type="number"
                      min={1}
                      required
                      value={roaming[key]}
                      onChange={(e) =>
                        change({
                          ...definition,
                          roaming: {
                            ...roaming,
                            [key]: Number(e.target.value),
                          },
                        })
                      }
                    />
                  </Field>
                ))}
              </div>
              {definition.steps.length > 0 && (
                <button
                  type="button"
                  onClick={() => change({ ...definition, steps: [] })}
                >
                  按上述配置重新生成路线（保存后形成新版本）
                </button>
              )}
            </>
          )}
        </fieldset>
      )}
    </>
  );
}
