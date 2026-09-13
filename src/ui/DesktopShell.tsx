import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Project } from "../shared/types";
import type { WindowCommand } from "../shared/window";
import { Icon } from "./Icon";
import { useNotify } from "./Notifications";
import { errorMessage } from "./notification-state";

export type Page =
  | "overview"
  | "tasks"
  | "assets"
  | "features"
  | "project"
  | "settings"
  | "environment"
  | "create"
  | "docs"
  | "library";
const titles = {
  overview: "游戏",
  create: "创作",
  docs: "资料",
  library: "素材",
  tasks: "任务",
  assets: "素材",
  features: "功能块",
  project: "项目设置",
  settings: "设置",
  environment: "创作环境",
};

export function DesktopShell({
  page,
  navigate,
  projects,
  projectId,
  selectProject,
  createProject,
  openProject,
  refresh,
  busy,
  running,
  ready,
  environment,
  actions,
  children,
}: {
  page: Page;
  navigate: (page: Page) => void;
  projects: Project[];
  projectId: string;
  selectProject: (id: string) => void;
  createProject: () => void;
  openProject: () => void;
  refresh: () => void;
  busy: boolean;
  running: number;
  ready: boolean;
  environment: () => void;
  actions: ReactNode;
  children: ReactNode;
}) {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return localStorage.getItem("beaver.railCollapsed") === "true";
    } catch {
      return false;
    }
  });
  const [maximized, setMaximized] = useState(false);
  const notify = useNotify();
  const previousPage = useRef<Page>("overview");
  useEffect(() => {
    if (page !== "settings") previousPage.current = page;
  }, [page]);
  useEffect(() => {
    try {
      localStorage.setItem("beaver.railCollapsed", String(collapsed));
    } catch {
      /* The shell remains usable if preference storage is unavailable. */
    }
  }, [collapsed]);
  useEffect(() => {
    const unsubscribe = window.beaver.onWindowState((state) =>
      setMaximized(state.maximized),
    );
    void window.beaver
      .windowControl("state")
      .then((state) => setMaximized(state.maximized))
      .catch((error: unknown) =>
        notify({ tone: "error", text: errorMessage(error) }),
      );
    return unsubscribe;
  }, [notify]);
  const control = (command: WindowCommand) => {
    void window.beaver
      .windowControl(command)
      .catch((error: unknown) =>
        notify({ tone: "error", text: errorMessage(error) }),
      );
  };
  const activePage =
    page === "tasks"
      ? "create"
      : page === "assets"
        ? "library"
        : ["features", "project"].includes(page)
          ? "overview"
          : page;
  const navigationItem = (id: Page) => (
    <button
      className={`rail-item${activePage === id ? " active" : ""}`}
      key={id}
      aria-label={titles[id]}
      title={titles[id]}
      aria-current={activePage === id ? "page" : undefined}
      onClick={() => navigate(id)}
    >
      <span className="rail-icon">
        <Icon
          name={
            id === "create"
              ? "tasks"
              : id === "docs"
                ? "book"
                : id === "library"
                  ? "assets"
                  : id === "environment"
                    ? "tools"
                    : id
          }
        />
      </span>
      <span className="rail-label">{titles[id]}</span>
      {id === "create" && running > 0 && (
        <b title={`${running} 个任务运行中`}>{running}</b>
      )}
    </button>
  );
  return (
    <div className={`app-shell${collapsed ? " rail-collapsed" : ""}`}>
      <aside className="rail">
        <div className="brand" data-native-drag-region>
          <button
            className="shell-icon-button rail-toggle"
            title={collapsed ? "展开侧栏" : "收起侧栏"}
            aria-label={collapsed ? "展开侧栏" : "收起侧栏"}
            aria-expanded={!collapsed}
            onClick={() => setCollapsed((value) => !value)}
          >
            <Icon name="sidebar" />
          </button>
          <img
            className="brand-mark"
            src="./beaver.svg"
            alt=""
            aria-hidden="true"
            draggable={false}
            width={28}
            height={28}
          />
          <strong>Beaver</strong>
        </div>
        <nav className="rail-nav" aria-label="工作区">
          {(["create", "docs", "library", "overview"] as const).map(
            navigationItem,
          )}
        </nav>
        <nav className="rail-footer" aria-label="应用">
          <button
            className={`rail-item${page === "environment" ? " active" : ""}`}
            aria-label="创作环境"
            aria-current={page === "environment" ? "page" : undefined}
            title="创作环境"
            onClick={environment}
          >
            <span className="rail-icon">
              <Icon name="tools" />
            </span>
            <span className="rail-label">创作环境</span>
          </button>
          {navigationItem("settings")}
        </nav>
      </aside>
      <header className="topbar">
        <div className="titlebar-context" data-native-drag-region>
          {page === "settings" ? (
            <button
              className="window-control titlebar-back"
              aria-label="返回工作区"
              title="返回工作区"
              onClick={() => navigate(previousPage.current)}
            >
              <Icon name="back" />
            </button>
          ) : (
            <div className="project-tools">
              <select
                className="project-select"
                aria-label="当前项目"
                value={projectId}
                title={
                  projects.find((project) => project.id === projectId)?.name ??
                  "选择项目"
                }
                disabled={!ready || !projects.length}
                onChange={(event) => selectProject(event.target.value)}
              >
                <option value="" disabled>
                  选择项目
                </option>
                {projects.map((project) => (
                  <option value={project.id} key={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
              <button
                className="shell-icon-button"
                aria-label="新建项目"
                title="新建项目"
                disabled={!ready}
                onClick={createProject}
              >
                <Icon name="add" />
              </button>
              <button
                className="shell-icon-button"
                aria-label="打开项目"
                title="打开项目"
                disabled={!ready}
                onClick={openProject}
              >
                <Icon name="folder" />
              </button>
            </div>
          )}
        </div>
        <div className="window-controls">
          <button
            className="window-control window-refresh"
            aria-label="刷新"
            title={busy ? "处理中…" : "刷新"}
            aria-busy={busy}
            disabled={busy}
            onClick={refresh}
          >
            <Icon name="refresh" />
          </button>
          <button
            className="window-control"
            aria-label="最小化"
            title="最小化"
            onClick={() => control("minimize")}
          >
            <Icon name="minimize" />
          </button>
          <button
            className="window-control"
            aria-label={maximized ? "还原窗口" : "最大化"}
            title={maximized ? "还原窗口" : "最大化"}
            onClick={() => control("toggleMaximize")}
          >
            <Icon name={maximized ? "restore" : "maximize"} />
          </button>
          <button
            className="window-control window-close"
            aria-label="关闭窗口"
            title="关闭窗口，任务在托盘继续"
            onClick={() => control("close")}
          >
            <Icon name="close" />
          </button>
        </div>
      </header>
      <main
        className={`main${["settings", "environment"].includes(page) ? " main-settings" : ""}${["create", "docs", "library"].includes(page) ? " main-studio" : ""}`}
      >
        {![
          "settings",
          "environment",
          "overview",
          "create",
          "docs",
          "library",
        ].includes(page) && (
          <div className="page-header">
            <h1>{titles[page]}</h1>
            {actions}
          </div>
        )}
        {children}
      </main>
    </div>
  );
}
