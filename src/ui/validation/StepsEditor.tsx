import { useState } from "react";
import { Field } from "../components";
import { defaultStep, type Step } from "./flow";

const kinds: Record<Step["kind"], string> = {
  wait: "等待帧",
  action: "游戏动作",
  key: "键盘按键",
  click: "点击节点",
  waitFor: "等待节点状态",
  capture: "采集画面",
};
function JsonValue({
  value,
  change,
}: {
  value: unknown;
  change: (value: unknown) => void;
}) {
  const [text, setText] = useState(JSON.stringify(value));
  const [invalid, setInvalid] = useState(false);
  return (
    <Field label="预期值（JSON）">
      <input
        value={text}
        aria-invalid={invalid}
        onChange={(e) => {
          setText(e.target.value);
          try {
            change(JSON.parse(e.target.value) as unknown);
            setInvalid(false);
          } catch {
            setInvalid(true);
          }
        }}
        required
        pattern={invalid ? "(?!)" : undefined}
      />
    </Field>
  );
}

export function StepsEditor({
  steps,
  change,
}: {
  steps: Step[];
  change: (steps: Step[]) => void;
}) {
  function replace(index: number, step: Step) {
    change(steps.map((old, i) => (i === index ? step : old)));
  }
  return (
    <fieldset className="validation-steps">
      <legend>动作与采集点</legend>
      <p className="muted">
        节点路径相对游戏场景；优先等待状态，再采集画面。按下动作后请添加释放动作。
      </p>
      {steps.map((step, index) => (
        <div className="validation-step" key={index}>
          <div className="validation-toolbar">
            <b>{index + 1}</b>
            <Field label="步骤标识">
              <input
                required
                value={step.id}
                onChange={(e) =>
                  replace(index, { ...step, id: e.target.value })
                }
              />
            </Field>
            <Field label="操作">
              <select
                value={step.kind}
                onChange={(e) =>
                  replace(index, {
                    ...defaultStep(e.target.value as Step["kind"], step.id),
                    references: step.references,
                  })
                }
              >
                {Object.entries(kinds).map(([kind, title]) => (
                  <option key={kind} value={kind}>
                    {title}
                  </option>
                ))}
              </select>
            </Field>
            <button
              type="button"
              disabled={!index}
              onClick={() => {
                const previous = steps[index - 1];
                if (!previous) return;
                const next = [...steps];
                next[index - 1] = step;
                next[index] = previous;
                change(next);
              }}
            >
              上移
            </button>
            <button
              type="button"
              onClick={() => change(steps.filter((_, i) => i !== index))}
            >
              移除
            </button>
          </div>
          <div className="validation-form-grid">
            {step.kind === "wait" && (
              <Field label="等待帧数">
                <input
                  type="number"
                  min={1}
                  required
                  value={step.frames}
                  onChange={(e) =>
                    replace(index, { ...step, frames: Number(e.target.value) })
                  }
                />
              </Field>
            )}
            {step.kind === "action" && (
              <Field label="Input Map 动作">
                <input
                  required
                  value={step.name}
                  onChange={(e) =>
                    replace(index, { ...step, name: e.target.value })
                  }
                />
              </Field>
            )}
            {step.kind === "key" && (
              <Field label="Godot Key 数值">
                <input
                  type="number"
                  min={1}
                  required
                  value={step.code}
                  onChange={(e) =>
                    replace(index, { ...step, code: Number(e.target.value) })
                  }
                />
              </Field>
            )}
            {(step.kind === "action" || step.kind === "key") && (
              <Field label="状态">
                <select
                  value={String(step.pressed)}
                  onChange={(e) =>
                    replace(index, {
                      ...step,
                      pressed: e.target.value === "true",
                    })
                  }
                >
                  <option value="true">按下</option>
                  <option value="false">释放</option>
                </select>
              </Field>
            )}
            {(step.kind === "click" || step.kind === "waitFor") && (
              <Field label="节点路径">
                <input
                  required
                  value={step.node}
                  onChange={(e) =>
                    replace(index, { ...step, node: e.target.value })
                  }
                />
              </Field>
            )}
            {step.kind === "waitFor" && (
              <>
                <Field label="属性">
                  <input
                    required
                    value={step.property}
                    onChange={(e) =>
                      replace(index, { ...step, property: e.target.value })
                    }
                  />
                </Field>
                <JsonValue
                  key={`${step.id}:${step.kind}`}
                  value={step.equals}
                  change={(equals) => replace(index, { ...step, equals })}
                />
                <Field label="超时帧数">
                  <input
                    type="number"
                    min={1}
                    required
                    value={step.timeout}
                    onChange={(e) =>
                      replace(index, {
                        ...step,
                        timeout: Number(e.target.value),
                      })
                    }
                  />
                </Field>
              </>
            )}
          </div>
        </div>
      ))}
      <div className="validation-toolbar">
        <button
          type="button"
          onClick={() => change([...steps, defaultStep("action")])}
        >
          添加动作
        </button>
        <button
          type="button"
          onClick={() => change([...steps, defaultStep("waitFor")])}
        >
          添加状态等待
        </button>
        <button
          type="button"
          onClick={() => change([...steps, defaultStep("capture")])}
        >
          添加采集点
        </button>
      </div>
    </fieldset>
  );
}
