import { Fragment, useState } from "react";
import { Icon } from "../Icon";
import { type DemoTask } from "./mock-tasks";
import { IterationComposer } from "./IterationComposer";
import { IterationProgress } from "./IterationProgress";
import { useIterationReorder } from "./useIterationReorder";
import type { IterationDraftState } from "./useIterationDraft";
import {
  insertMockIteration,
  iterationOrder,
  objectIterations,
  updateMockIteration,
} from "./mock-iterations";
import "./preview-iteration-stack.css";

export function ObjectIterationStack({
  objectId,
  activeTask,
  iteration,
  openWorkflow,
  iterations: providedIterations,
}: {
  objectId: string;
  activeTask: DemoTask | null;
  iteration: IterationDraftState;
  openWorkflow: (task: DemoTask) => void;
  iterations?: DemoTask[];
}) {
  const [expanded, setExpanded] = useState<string[]>([]);
  const draft = iteration.draft?.objectId === objectId ? iteration.draft : null;
  const readOnly = providedIterations !== undefined;
  const iterations = providedIterations ?? objectIterations(objectId);
  const reorder = useIterationReorder(objectId, iteration.stopAnnotating);
  const dragging = iterations.find(
    (task) => task.id === reorder.preview?.taskId,
  );
  const current =
    iterations.find((task) => task.id === activeTask?.id) ??
    iterations.find(
      (task) => task.parentId === activeTask?.id && iterationOrder(task) === 1,
    ) ??
    iterations.find((task) => iterationOrder(task) === 1);

  if (!iterations.length) {
    return <p className="op-muted">暂无修改建议</p>;
  }

  const renderInsertion = (anchor: DemoTask, before = false) => {
    const afterId = before ? null : anchor.id;
    const isInserting = draft?.taskId === null && draft.afterId === afterId;
    return (
      <li
        className={
          isInserting ? "op-iteration-item is-draft" : "op-iteration-insert"
        }
      >
        {isInserting ? (
          <IterationComposer
            iteration={iteration}
            submit={() => {
              if (!draft) return;
              const inserted = insertMockIteration(
                anchor,
                draft,
                before ? "before" : "after",
              );
              if (!inserted) return;
              setExpanded((value) => [...value, inserted.id]);
              iteration.cancel();
            }}
          />
        ) : (
          <button
            type="button"
            className="op-iteration-add"
            title="在此插入任务"
            aria-label={`在${anchor.summary ?? anchor.title}${before ? "之前" : "之后"}插入任务`}
            onClick={() => iteration.begin(objectId, afterId)}
          >
            <Icon name="add" />
          </button>
        )}
      </li>
    );
  };

  const renderDropPreview = (beforeId: string | null) =>
    dragging &&
    reorder.preview?.valid &&
    reorder.preview.beforeId === beforeId ? (
      <li className="op-iteration-drop-preview" aria-hidden="true">
        <div className="op-iteration-heading">
          <IterationProgress task={dragging} />
          <strong className="op-iteration-title">
            {dragging.summary ?? dragging.title}
          </strong>
          <Icon name="grip" />
        </div>
      </li>
    ) : null;

  return (
    <ol
      ref={reorder.listRef}
      className={`op-iteration-stack${dragging ? " is-reordering" : ""}`}
      aria-label="迭代栈"
    >
      {!readOnly && renderInsertion(iterations[0]!, true)}
      {iterations.map((task) => {
        const isExpanded = expanded.includes(task.id);
        return (
          <Fragment key={task.id}>
            {!readOnly && renderDropPreview(task.id)}
            <li
              className={`op-iteration-item${current?.id === task.id ? " is-current" : ""}${dragging?.id === task.id ? " is-dragging" : ""}`}
              data-iteration-id={task.id}
              aria-current={current?.id === task.id ? "step" : undefined}
            >
              <IterationComposer
                task={task}
                iteration={iteration}
                expanded={isExpanded}
                openWorkflow={readOnly ? undefined : () => openWorkflow(task)}
                dragHandle={readOnly ? undefined : reorder.handle(task.id)}
                readOnly={readOnly}
                toggle={() =>
                  setExpanded((value) =>
                    isExpanded
                      ? value.filter((id) => id !== task.id)
                      : [...value, task.id],
                  )
                }
                submit={() => {
                  if (draft?.taskId !== task.id) return;
                  if (updateMockIteration(task, draft)) iteration.cancel();
                }}
              />
            </li>
            {!readOnly && renderInsertion(task)}
          </Fragment>
        );
      })}
      {!readOnly && renderDropPreview(null)}
    </ol>
  );
}
