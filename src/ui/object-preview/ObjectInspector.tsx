import { Icon } from "../Icon";
import { type DemoObject } from "./mock-objects";
import { type DemoTask } from "./mock-tasks";
import { ObjectInspectorHeader } from "./ObjectInspectorHeader";
import { ObjectInspectorPreview } from "./ObjectInspectorPreview";
import { ObjectIterationStack } from "./ObjectIterationStack";
import { type PreviewMode } from "./preview-target";
import type { ObjectPreviewState } from "./useObjectPreview";
import type { IterationDraftState } from "./useIterationDraft";
import "./preview-inspector.css";

export function ObjectInspector({
  object,
  task,
  preview,
  iteration,
  mode,
  setMode,
  inspect,
  expanded,
  openWorkflow,
  notify,
  iterations,
}: {
  object: DemoObject | null;
  task: DemoTask | null;
  preview: ObjectPreviewState;
  iteration: IterationDraftState;
  mode: PreviewMode;
  setMode: (mode: PreviewMode) => void;
  inspect: () => void;
  expanded: boolean;
  openWorkflow: (task: DemoTask) => void;
  notify: (message: string) => void;
  iterations?: DemoTask[];
}) {
  const { tab, feedback } = preview;
  return (
    <aside
      id="op-object-inspector"
      className="op-object-inspector"
      aria-label="对象详情"
      data-object-id={object?.id}
      data-task-id={task?.id}
      hidden={!expanded}
    >
      {object ? (
        <>
          <ObjectInspectorHeader
            object={object}
            task={task}
            preview={preview}
          />
          <div className="op-inspector-scroll" role="region" aria-label={tab}>
            {tab === "迭代栈" ? (
              <ObjectIterationStack
                objectId={object.id}
                activeTask={task}
                iteration={iteration}
                openWorkflow={openWorkflow}
                iterations={iterations}
              />
            ) : tab === "预览与反馈" ? (
              <ObjectInspectorPreview
                feedback={feedback}
                mode={mode}
                setMode={setMode}
                change={preview.selectFeedback}
                inspect={inspect}
                notify={notify}
              />
            ) : (
              <div className="op-versions">
                <p className="op-muted">演示快照 · 原有版本保留</p>
                {[
                  object.version,
                  ...(object.accepted && object.accepted !== object.version
                    ? [object.accepted]
                    : []),
                ].map((version) => (
                  <div key={version}>
                    <Icon name="layers" />
                    <span>
                      <strong>{version}</strong>
                      <small>
                        {version === object.accepted
                          ? "已验收版本"
                          : "当前工作版本"}
                      </small>
                    </span>
                    <button onClick={() => preview.selectVersion(version)}>
                      查看
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      ) : (
        <div className="op-empty">
          <Icon name="layers" />
          <h2>尚未选择对象</h2>
          <p>选择没有进行中任务的对象，在这里查看其内容。</p>
        </div>
      )}
    </aside>
  );
}
