import { useId } from "react";
import { Icon } from "../Icon";
import { IterationAnnotations } from "./IterationAnnotations";
import { IterationProgress } from "./IterationProgress";
import type { DemoTask } from "./mock-tasks";
import type { IterationDraftState } from "./useIterationDraft";
import type { IterationDragHandle } from "./useIterationReorder";

export function IterationComposer({
  submit,
  iteration,
  task,
  expanded = true,
  toggle,
  openWorkflow,
  dragHandle,
  readOnly = false,
}: {
  submit: () => void;
  iteration: IterationDraftState;
  task?: DemoTask;
  expanded?: boolean;
  toggle?: () => void;
  openWorkflow?: () => void;
  dragHandle?: IterationDragHandle;
  readOnly?: boolean;
}) {
  const detailsId = useId();
  const { draft, cancel, setPrompt, toggleMode, edit, remove } = iteration;
  const editingDraft =
    draft && (task ? draft.taskId === task.id : !draft.taskId) ? draft : null;
  const editing = !!editingDraft;
  const locked = readOnly || (!!task && !editing);
  const content =
    editingDraft ??
    (task && {
      title: task.title,
      prompt: task.prompt ?? task.detail,
      annotations: task.annotations ?? [],
    });
  if (!content) return null;
  const { title, prompt, annotations } = content;
  const mode = editingDraft?.mode ?? null;
  const canSubmit =
    !!prompt.trim() || annotations.some((value) => value.prompt.trim());
  return (
    <form
      className="op-iteration-compose"
      aria-label={task ? `迭代任务：${task.title}` : "插入迭代任务"}
      data-locked={locked}
      onSubmit={(event) => {
        event.preventDefault();
        if (!locked && canSubmit) submit();
      }}
      onKeyDown={(event) => {
        if (event.nativeEvent.isComposing) return;
        if (editing && event.key === "Escape") {
          event.stopPropagation();
          cancel();
        }
      }}
    >
      <header className="op-iteration-heading">
        {task ? (
          <IterationProgress task={task} />
        ) : (
          <span className="op-iteration-draft-icon" aria-hidden="true">
            <Icon name="add" />
          </span>
        )}
        {expanded ? (
          <div className="op-iteration-name">
            <input
              aria-label="任务名称"
              placeholder="任务名称（留空自动生成）"
              title={title || "可手动填写，留空时自动生成名称"}
              value={title}
              readOnly={locked}
              onChange={(event) => iteration.setTitle(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.nativeEvent.isComposing)
                  event.preventDefault();
              }}
            />
            {!locked && (
              <button
                type="button"
                className="op-iteration-name-generate"
                aria-label="AI 生成任务名称"
                title="AI 生成任务名称（演示）"
                disabled={!canSubmit}
                onClick={iteration.generateTitle}
              >
                <Icon name="sparkles" />
              </button>
            )}
          </div>
        ) : (
          <strong className="op-iteration-title" title={title}>
            {editing ? title || "未命名任务" : (task?.summary ?? title)}
          </strong>
        )}
        {expanded && (
          <div
            className="op-annotation-tools"
            role="group"
            aria-label="位置标注工具"
          >
            <button
              type="button"
              disabled={locked}
              aria-pressed={mode === "point"}
              onClick={() => toggleMode("point")}
            >
              <Icon name="target" />
              点选
            </button>
            <button
              type="button"
              disabled={locked}
              aria-pressed={mode === "box"}
              onClick={() => toggleMode("box")}
            >
              <Icon name="maximize" />
              框选
            </button>
          </div>
        )}
        {task && dragHandle && (
          <button
            type="button"
            className="op-iteration-drag"
            title="拖动调整顺序，也可按上下方向键"
            aria-label={`拖动${task.title}调整顺序`}
            {...dragHandle}
          >
            <Icon name="grip" />
          </button>
        )}
        {task && !readOnly && (
          <button
            type="button"
            className="op-iteration-lock"
            title={locked ? "已锁定，点击解锁编辑" : "保存修改并锁定"}
            aria-label={locked ? `解锁编辑${task.title}` : "保存修改并锁定"}
            aria-pressed={locked}
            disabled={!locked && !canSubmit}
            onClick={() => {
              if (locked) {
                iteration.beginEdit(task);
                if (!expanded) toggle?.();
              } else if (canSubmit) submit();
            }}
          >
            <Icon name={locked ? "lock" : "unlock"} />
          </button>
        )}
        {task && openWorkflow && (
          <button
            type="button"
            className="op-iteration-jump"
            title="打开任务流程编排"
            aria-label={`打开${task.title}的流程编排`}
            onClick={openWorkflow}
          >
            <Icon name="arrowUpRight" />
          </button>
        )}
        {toggle && (
          <button
            type="button"
            className="op-iteration-toggle"
            aria-label={`${expanded ? "收起" : "展开"}${title}`}
            aria-expanded={expanded}
            aria-controls={detailsId}
            onClick={() => {
              if (editing) iteration.stopAnnotating();
              toggle();
            }}
          >
            <Icon name="chevronDown" />
          </button>
        )}
      </header>
      <div
        className="op-iteration-compose-body"
        id={detailsId}
        hidden={!expanded}
      >
        {mode && (
          <p className="op-annotation-hint" role="status">
            {mode === "point"
              ? "点击左侧对象或子对象画面添加编号"
              : "在左侧对象或子对象画面拖动框选"}
            ，再次点击工具可退出。
          </p>
        )}
        <IterationAnnotations
          annotations={annotations}
          edit={locked ? undefined : edit}
          remove={locked ? undefined : remove}
          gallery
        />
        <textarea
          autoFocus={!task}
          aria-label="任务需求"
          placeholder="描述整体需求，例如：把序号 1、2、4 的颜色变为黄色…"
          rows={4}
          value={prompt}
          readOnly={locked}
          onChange={(event) => setPrompt(event.target.value)}
        />
        {!locked && (
          <footer>
            <button type="button" onClick={cancel}>
              {task ? "取消修改" : "取消"}
            </button>
            <button type="submit" className="primary" disabled={!canSubmit}>
              {task ? "保存并锁定" : "提交"}
            </button>
          </footer>
        )}
      </div>
    </form>
  );
}
