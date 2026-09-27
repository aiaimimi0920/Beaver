import { useState } from "react";
import { Icon, type IconName } from "../Icon";
import { ManufactureView } from "./ManufactureView";
import { ObjectLibrary } from "./ObjectLibrary";
import { DemoActionDialog } from "./PreviewControls";
import { TaskDetail } from "./TaskDetail";
import { TaskLanes } from "./TaskLanes";
import type { ObjectTaskSnapshot } from "../../shared/object-tasks";
import { previewTasksFromSnapshot } from "./object-task-preview-adapter";
import { objectById, type PreviewPage } from "./mock-objects";
import { resetMockIterations } from "./mock-iterations";
import "./preview-shell.css";
import "./preview-library.css";
import "./preview-tasks.css";
import "./preview-creation.css";
import "./preview-controls.css";
import "./preview-embedded.css";

const pages: {
  id: PreviewPage;
  label: string;
  detail: string;
  icon: IconName;
}[] = [
  { id: "tasks", label: "创作", detail: "目标与任务", icon: "tasks" },
  {
    id: "objects",
    label: "对象",
    detail: "项目中的所有对象",
    icon: "features",
  },
  { id: "creation", label: "制造", detail: "制作流程与交付", icon: "layers" },
];

export function ObjectFrameworkPreview({
  embedded = false,
  showPanelTabs = true,
  activePage,
  onPageChange,
  taskSnapshot,
}: {
  embedded?: boolean;
  showPanelTabs?: boolean;
  activePage?: PreviewPage;
  onPageChange?: (page: PreviewPage) => void;
  taskSnapshot?: ObjectTaskSnapshot;
}) {
  const [localPage, setLocalPage] = useState<PreviewPage>("objects");
  const page = activePage ?? localPage;
  const setPage = (next: PreviewPage) => {
    setLocalPage(next);
    onPageChange?.(next);
  };
  const [objectId, setObjectId] = useState("O-001");
  const [stage, setStage] = useState<number>();
  const [version, setVersion] = useState<string>();
  const [taskId, setTaskId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{
    title: string;
    context: string;
  } | null>(null);
  const [notice, setNotice] = useState("");
  const [resetKey, setResetKey] = useState(0);
  const selected = objectById(objectId);
  const previewTasks = taskSnapshot
    ? previewTasksFromSnapshot(taskSnapshot)
    : undefined;
  const openObject = (id: string) => {
    setObjectId(id);
    setPage("objects");
    setTaskId(null);
  };
  const openCreation = (
    id: string,
    nextStage?: number,
    nextVersion?: string,
  ) => {
    setObjectId(id);
    setStage(nextStage);
    setVersion(nextVersion);
    setPage("creation");
    setTaskId(null);
  };
  const action = (title: string, context = "") => {
    setTaskId(null);
    setDialog({ title, context });
  };
  const reset = () => {
    resetMockIterations();
    setPage("objects");
    setObjectId("O-001");
    setStage(undefined);
    setVersion(undefined);
    setTaskId(null);
    setDialog(null);
    setResetKey((value) => value + 1);
    setNotice("演示已重置 · 未写入项目，未启动任何制作任务。");
  };
  return (
    <div className={`op-shell${embedded ? " op-embedded" : ""}`}>
      <div className="op-workspace" inert={taskId !== null || dialog !== null}>
        {!embedded && (
          <aside className="op-sidebar">
            <div className="op-brand">
              <img src="./beaver.svg" alt="" />
              <strong>
                Beaver<span>CREATION STUDIO</span>
              </strong>
            </div>
            <button
              className="op-project-switch"
              onClick={() => action("切换项目", "演示项目列表")}
            >
              <span className="op-project-avatar">放</span>
              <span>
                <strong>放课后</strong>
                <small>After School</small>
              </span>
              <span>⌄</span>
            </button>
            <p className="op-nav-label">创作空间</p>
            <nav aria-label="创作空间">
              {pages.map((item) => (
                <button
                  key={item.id}
                  aria-current={page === item.id ? "page" : undefined}
                  onClick={() => {
                    setPage(item.id);
                    setStage(undefined);
                    setVersion(undefined);
                  }}
                >
                  <Icon name={item.icon} />
                  <span>
                    {item.label}
                    <small>{item.detail}</small>
                  </span>
                  {item.id === "tasks" && <em>8</em>}
                </button>
              ))}
            </nav>
            <div className="op-sidebar-bottom">
              <div className="op-demo-indicator">
                <i />
                <span>
                  界面预览模式<small>无运行中的工具</small>
                </span>
              </div>
              <button onClick={() => action("项目设置", "设置界面占位")}>
                <Icon name="settings" />
                项目设置
              </button>
              <span className="op-mono">BEAVER / UI STUDY 01</span>
            </div>
          </aside>
        )}
        <div className="op-main">
          {page === "tasks" && (
            <header className="op-topbar">
              <span>
                <Icon name="project" />
                演示项目：放课后<span className="op-topbar-divider">/</span>
                创作工作台
              </span>
              <div>
                <span className="op-demo-badge">UI 预览 · 测试数据</span>
                <button onClick={reset} title="重置演示界面">
                  <Icon name="refresh" />
                  重置演示
                </button>
              </div>
            </header>
          )}
          {embedded && showPanelTabs && (
            <nav className="op-panel-tabs" aria-label="创作空间">
              {pages.map((item) => (
                <button
                  key={item.id}
                  aria-current={page === item.id ? "page" : undefined}
                  onClick={() => {
                    setPage(item.id);
                    setStage(undefined);
                    setVersion(undefined);
                  }}
                >
                  <Icon name={item.icon} />
                  {item.label}
                  <small>{item.detail}</small>
                </button>
              ))}
            </nav>
          )}
          <div className="op-main-content" key={resetKey}>
            {page === "objects" ? (
              <ObjectLibrary
                selected={selected}
                select={setObjectId}
                openCreation={openCreation}
                notify={setNotice}
                taskSnapshot={taskSnapshot}
              />
            ) : page === "tasks" ? (
              <TaskLanes
                openTask={setTaskId}
                openCreation={openCreation}
                action={action}
                tasks={previewTasks}
              />
            ) : (
              <ManufactureView
                object={selected}
                initialStage={stage}
                initialVersion={version}
                openCreation={openCreation}
              />
            )}
          </div>
          <footer className="op-statusbar">
            <span role="status" aria-live="polite">
              {notice && <i />}
              {notice}
            </span>
            <span aria-hidden="true" />
          </footer>
        </div>
      </div>
      {taskId && (
        <TaskDetail
          key={taskId}
          id={taskId}
          close={() => setTaskId(null)}
          openTask={setTaskId}
          openObject={openObject}
          openCreation={openCreation}
          action={action}
        />
      )}
      {dialog && (
        <DemoActionDialog
          key={dialog.title}
          title={dialog.title}
          context={dialog.context}
          close={() => setDialog(null)}
          notify={setNotice}
        />
      )}
    </div>
  );
}
