import { useState } from "react";
import { Icon } from "../Icon";
import { IterationAnnotations } from "./IterationAnnotations";
import { IterationProgress } from "./IterationProgress";
import type { DemoObject } from "./mock-objects";
import type { DemoTask } from "./mock-tasks";
import { mockWorkflow, workflowStepStatus } from "./mock-workflow";
import "./preview-workflow.css";

export function TaskWorkflowView({
  task,
  object,
  hasUnsavedChanges,
  close,
}: {
  task: DemoTask;
  object: DemoObject;
  hasUnsavedChanges: boolean;
  close: () => void;
}) {
  const steps = mockWorkflow(task, object);
  const [selected, setSelected] = useState(() =>
    Math.min(
      steps.length - 1,
      Math.floor((task.progress / 100) * steps.length),
    ),
  );
  const step = steps[selected];
  const status = workflowStepStatus(task, selected, steps.length);
  if (!step)
    return (
      <section className="op-page op-workflow" aria-label="任务流程编排">
        <button onClick={close}>返回迭代栈</button>
        <p>尚无可显示的制作步骤。</p>
      </section>
    );
  return (
    <section className="op-page op-workflow" aria-label="任务流程编排">
      <header className="op-workflow-header">
        <button type="button" autoFocus onClick={close}>
          <Icon name="back" />
          返回迭代栈
        </button>
        <span className="op-workflow-location">
          {object.name} / {task.id}
        </span>
        <span className="op-demo-badge">UI 演示 · 流程与调用均为示例</span>
      </header>
      <div className="op-workflow-title">
        <Icon name="tasks" />
        <div>
          <h1>任务流程编排</h1>
          <p>{task.title}</p>
        </div>
        <IterationProgress task={task} />
      </div>
      <div className="op-workflow-layout">
        <aside className="op-workflow-goal">
          <h2>
            <Icon name="speech" />
            人类目标
          </h2>
          <dl className="op-workflow-context">
            <div>
              <dt>来源任务</dt>
              <dd>{task.id}</dd>
            </div>
            <div>
              <dt>操作对象</dt>
              <dd>
                {object.id} · {object.name}
              </dd>
            </div>
            <div>
              <dt>对象版本</dt>
              <dd>{object.version}</dd>
            </div>
          </dl>
          <h3>{task.title}</h3>
          <p className="op-workflow-prompt">{task.prompt ?? task.detail}</p>
          <IterationAnnotations annotations={task.annotations ?? []} />
          {hasUnsavedChanges && (
            <p className="op-workflow-draft-note" role="status">
              当前展示已保存的目标。未保存的编辑仍保留在迭代栈中，返回后可继续修改。
            </p>
          )}
        </aside>
        <nav className="op-workflow-stages" aria-label="制作流程步骤">
          <h2>
            <Icon name="sparkles" />
            Codex 制作流程
          </h2>
          <p className="op-workflow-muted">前一步交付完成后，推进下一步</p>
          <ol>
            {steps.map((item, index) => {
              const state = workflowStepStatus(task, index, steps.length);
              return (
                <li key={item.skill}>
                  <button
                    type="button"
                    className="op-workflow-step"
                    aria-current={selected === index ? "step" : undefined}
                    aria-controls="op-workflow-step-detail"
                    data-status={state}
                    onClick={() => setSelected(index)}
                  >
                    <span className="op-workflow-step-number">
                      {state === "已完成"
                        ? "✓"
                        : String(index + 1).padStart(2, "0")}
                    </span>
                    <span>
                      <strong>{item.title}</strong>
                      <small>{state}</small>
                    </span>
                    <Icon name="back" />
                  </button>
                </li>
              );
            })}
          </ol>
        </nav>
        <article
          className="op-workflow-detail"
          id="op-workflow-step-detail"
          aria-label="步骤详情"
        >
          <header>
            <span className="op-workflow-muted">
              步骤 {String(selected + 1).padStart(2, "0")} / {steps.length}
            </span>
            <span className="op-workflow-state">{status}</span>
          </header>
          <h2>{step.title}</h2>
          <p className="op-workflow-prompt">{step.instruction}</p>
          <div className="op-workflow-dependency">
            <Icon name="lock" />
            {selected === 0
              ? "前置条件：人类目标已提交并锁定"
              : `前置交付：${steps[selected - 1]?.title ?? "等待前序步骤"}`}
          </div>
          <section className="op-workflow-calls">
            <h3>
              <Icon name="tools" />
              工具调用链
            </h3>
            <p className="op-workflow-skill">
              <span>Skill 示例</span>
              <code>{step.skill}/SKILL.md</code>
            </p>
            <ol>
              {step.calls.map((call, index) => (
                <li key={call}>
                  <span>{index + 1}</span>
                  {call}
                </li>
              ))}
            </ol>
          </section>
          <div className="op-workflow-io">
            <section>
              <h3>输入</h3>
              <ul>
                {step.inputs.map((input, index) => (
                  <li key={index}>{input}</li>
                ))}
              </ul>
            </section>
            <section>
              <h3>输出 / 交付</h3>
              <ul>
                {step.outputs.map((output) => (
                  <li key={output}>{output}</li>
                ))}
              </ul>
            </section>
          </div>
          <section className="op-workflow-acceptance">
            <h3>
              <Icon name="review" />
              步骤完成条件
            </h3>
            <p>{step.acceptance}</p>
          </section>
        </article>
      </div>
    </section>
  );
}
