import { useRef, useState } from "react";
import { Icon } from "../Icon";
import { demoObjects, type DemoObject, type ObjectType } from "./mock-objects";
import { ObjectContentsDrawer } from "./ObjectContentsDrawer";
import { ObjectInspector } from "./ObjectInspector";
import { ObjectTaskRail } from "./ObjectTaskRail";
import { ObjectToolbar } from "./ObjectToolbar";
import { useObjectTaskNavigation } from "./useObjectTaskNavigation";
import { useObjectPreview } from "./useObjectPreview";
import { useIterationDraft } from "./useIterationDraft";
import { SelectablePreview } from "./SelectablePreview";
import { PreviewInspectDialog } from "./PreviewInspectDialog";
import { TaskWorkflowView } from "./TaskWorkflowView";
import type { DemoTask } from "./mock-tasks";
import {
  mainPreview,
  type PreviewFeedback,
  type PreviewMode,
} from "./preview-target";
import "./preview-selection.css";

export function ObjectLibrary({
  selected,
  select,
  openCreation,
  notify,
}: {
  selected: DemoObject;
  select: (id: string) => void;
  openCreation: (id: string, stage?: number, version?: string) => void;
  notify: (message: string) => void;
}) {
  const [filter, setFilter] = useState<ObjectType | "全部">("全部");
  const [search, setSearch] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [thumbnailSize, setThumbnailSize] = useState(290);
  const [inspectorExpanded, setInspectorExpanded] = useState(false);
  const [mode, setMode] = useState<PreviewMode>("point");
  const [inspectOpen, setInspectOpen] = useState(false);
  const [workflowTask, setWorkflowTask] = useState<DemoTask | null>(null);
  const workflowTrigger = useRef<HTMLElement | null>(null);
  const workflowObject = demoObjects.find(
    (object) => object.id === workflowTask?.objectId,
  );
  const navigation = useObjectTaskNavigation(selected, select);
  const iteration = useIterationDraft();
  const preview = useObjectPreview(
    navigation.inspectorObject ?? selected,
    !!iteration.draft,
  );
  const captureMode =
    inspectorExpanded && preview.tab === "迭代栈"
      ? (iteration.draft?.mode ?? null)
      : null;
  const { feedback } = preview;
  const selectObject = (id: string) => {
    iteration.cancel();
    preview.reset();
    setInspectOpen(false);
    navigation.selectObject(id);
  };
  const selectFeedback = (value: PreviewFeedback) => {
    if (captureMode) {
      if (
        !preview.choices.some(
          (choice) => choice.target.key === value.target.key,
        )
      ) {
        notify("请在当前对象或其子对象画面中标注");
        return;
      }
      iteration.add(value);
    }
    preview.selectFeedback(value);
    setInspectorExpanded(true);
  };
  const inspect = () => setInspectOpen(true);
  const objects = demoObjects.filter(
    (object) =>
      (filter === "全部" || object.objectType === filter) &&
      (!tags.length || tags.some((tag) => object.tags.includes(tag))) &&
      `${object.name} ${object.description} ${object.tags.join(" ")}`
        .toLowerCase()
        .includes(search.trim().toLowerCase()),
  );
  return (
    <>
      <section className="op-page op-object-library" hidden={!!workflowTask}>
        <ObjectToolbar
          filter={filter}
          setFilter={setFilter}
          search={search}
          setSearch={setSearch}
          tags={tags}
          setTags={setTags}
          thumbnailSize={thumbnailSize}
          setThumbnailSize={setThumbnailSize}
          mode={captureMode}
          canAnnotate={
            inspectorExpanded && preview.tab === "迭代栈" && !!iteration.draft
          }
          setMode={iteration.toggleMode}
          inspect={inspect}
          notify={notify}
        />
        <div
          className="op-library-layout"
          data-inspector-expanded={inspectorExpanded}
        >
          <div className="op-library-main">
            <div className="op-library-scroll">
              <div
                className="op-object-grid"
                style={{
                  gridTemplateColumns: `repeat(auto-fill, minmax(0, min(100%, ${thumbnailSize}px)))`,
                }}
              >
                {objects.map((object) => {
                  const target = mainPreview(preview.view(object));
                  return (
                    <SelectablePreview
                      key={target.key}
                      target={target}
                      className="op-object-card"
                      selected={object.id === selected.id}
                      mode={captureMode}
                      annotations={iteration.draft?.annotations}
                      annotating={!!captureMode}
                      selection={
                        feedback?.target.key === target.key
                          ? feedback.selection
                          : undefined
                      }
                      onSelect={(value) => {
                        if (!captureMode) {
                          if (object.id !== selected.id) iteration.cancel();
                          navigation.selectObject(object.id);
                        }
                        selectFeedback(value);
                      }}
                      open={() => openCreation(object.id)}
                    />
                  );
                })}
              </div>
              {!objects.length && (
                <div className="op-empty">
                  <Icon name="assets" />
                  <h2>没有匹配的对象</h2>
                  <button
                    onClick={() => {
                      setFilter("全部");
                      setSearch("");
                      setTags([]);
                    }}
                  >
                    清除筛选
                  </button>
                </div>
              )}
            </div>
            <ObjectContentsDrawer
              object={preview.view(selected)}
              select={selectObject}
              mode={captureMode}
              annotations={iteration.draft?.annotations}
              annotating={!!captureMode}
              feedback={feedback}
              onSelect={selectFeedback}
            />
          </div>
          <ObjectInspector
            key={
              navigation.activeTask?.id ??
              navigation.inspectorObject?.id ??
              "empty"
            }
            object={navigation.inspectorObject}
            task={navigation.activeTask}
            preview={preview}
            iteration={iteration}
            mode={mode}
            setMode={setMode}
            inspect={inspect}
            expanded={inspectorExpanded}
            openWorkflow={(task) => {
              workflowTrigger.current =
                document.activeElement instanceof HTMLElement
                  ? document.activeElement
                  : null;
              iteration.stopAnnotating();
              setWorkflowTask(task);
            }}
            notify={notify}
          />
          <ObjectTaskRail
            expanded={inspectorExpanded}
            toggle={() => setInspectorExpanded((value) => !value)}
            activeTaskId={navigation.activeTask?.id ?? null}
            slotObject={navigation.slotObject}
            selectSlot={() => {
              iteration.cancel();
              preview.reset();
              navigation.selectSlot();
              setInspectorExpanded(true);
            }}
            selectTask={(task) => {
              iteration.cancel();
              preview.reset();
              navigation.selectTask(task);
              setInspectorExpanded(true);
            }}
          />
        </div>
        {inspectOpen && (
          <PreviewInspectDialog
            initial={feedback}
            close={() => setInspectOpen(false)}
            change={selectFeedback}
            notify={notify}
          />
        )}
      </section>
      {workflowTask && workflowObject && (
        <TaskWorkflowView
          task={workflowTask}
          object={workflowObject}
          hasUnsavedChanges={iteration.draft?.taskId === workflowTask.id}
          close={() => {
            setWorkflowTask(null);
            requestAnimationFrame(() => workflowTrigger.current?.focus());
          }}
        />
      )}
    </>
  );
}
