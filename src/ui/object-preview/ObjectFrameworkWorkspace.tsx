import { useEffect, useRef, useState } from "react";
import type { Project } from "../../shared/types";
import type { ObjectAttemptCheckRequest } from "../../shared/object-attempt-checks";
import { Icon } from "../Icon";
import { ObjectToolbarControls } from "./ObjectToolbarControls";
import { ObjectImportDialog } from "./ObjectImportDialog";
import { ObjectGenerateDialog } from "./ObjectGenerateDialog";
import type { ObjectType } from "./object-categories";
import { ObjectCatalogGrid } from "./ObjectCatalogGrid";
import { useObjectCatalog } from "./use-object-catalog";
import { ObjectCatalogDetails } from "./ObjectCatalogDetails";
import { useObjectTaskQuery } from "../object-tasks/use-object-task-query";
import { ObjectProductionTasks } from "./ObjectProductionTasks";
import { ObjectTaskExecution } from "../object-tasks/object-task-execution";
import { ObjectTaskExecutionDialog } from "../object-tasks/ObjectTaskExecutionDialog";
import { call } from "../api";

export function ObjectFrameworkWorkspace({
  project,
  projects = [],
  loading,
  page,
  createProject,
  navigate,
  initialCheck,
}: {
  project?: Project;
  projects?: readonly Project[];
  loading: boolean;
  page: "objects" | "manufacture";
  createProject: () => void;
  navigate: (page: "objects" | "manufacture") => void;
  initialCheck?: ObjectAttemptCheckRequest;
}) {
  const [filter, setFilter] = useState<ObjectType | "全部">("全部");
  const [search, setSearch] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [thumbnailSize, setThumbnailSize] = useState(240);
  const [notice, setNotice] = useState("");
  const [selectedObject, setSelectedObject] = useState<string>();
  const [selectedTask, setSelectedTask] = useState<string>();
  const [tasksOpen, setTasksOpen] = useState(false);
  const [execution, setExecution] = useState<ObjectTaskExecution | null>(null);
  const selectObject = (id: string) => {
    setSelectedObject(id);
    setSelectedTask(undefined);
    setTasksOpen(true);
  };
  const [importTarget, setImportTarget] = useState<string>();
  const [generationTarget, setGenerationTarget] = useState<string>();
  const [catalogRevision, setCatalogRevision] = useState(0);
  const catalog = useObjectCatalog(project?.id, search, catalogRevision);
  const { state: taskQuery, refresh: refreshTasks } = useObjectTaskQuery(
    project?.id,
  );
  const openedCheck = useRef<ObjectAttemptCheckRequest | undefined>(undefined);
  useEffect(() => {
    if (
      !initialCheck ||
      taskQuery.kind !== "ready" ||
      openedCheck.current === initialCheck
    )
      return;
    openedCheck.current = initialCheck;
    const { target } = initialCheck;
    const task = taskQuery.snapshot.tasks.find(
      (item) => item.id === target.taskId,
    );
    if (
      initialCheck.projectId !== project?.id ||
      !task ||
      task.objectId !== target.objectId ||
      task.runId !== target.runId
    ) {
      setNotice("报告对应的制造任务不存在或身份已变化，请重新选择记录。");
      return;
    }
    try {
      setExecution(
        new ObjectTaskExecution(
          project.id,
          taskQuery.snapshot,
          task.id,
          call,
          async () => {
            setCatalogRevision((value) => value + 1);
            return refreshTasks();
          },
        ),
      );
      setSelectedObject(target.objectId);
      setSelectedTask(target.taskId);
      setTasksOpen(true);
    } catch (error: unknown) {
      setNotice(String(error));
    }
  }, [initialCheck, project?.id, taskQuery, refreshTasks]);
  const taskCount =
    taskQuery.kind === "ready" ? taskQuery.snapshot.tasks.length : undefined;
  const availableTags =
    catalog.kind === "ready"
      ? Array.from(
          new Map(
            catalog.objects
              .flatMap((object) => object.tags)
              .map((tag) => [tag.toLocaleLowerCase(), tag] as const),
          ).values(),
        ).sort((left, right) => left.localeCompare(right, "zh-Hans-CN"))
      : [];
  const selectedObjectRecord =
    selectedObject && catalog.kind === "ready"
      ? catalog.objects.find((object) => object.id === selectedObject)
      : undefined;
  const message = loading ? (
    <section className="op-framework-message" role="status">
      正在打开本地工作室…
    </section>
  ) : !project ? (
    <section className="op-framework-message">
      <h2>尚未选择项目</h2>
      <p>创建或选择项目后查看对象与制造记录。</p>
      <button onClick={createProject}>创建项目</button>
    </section>
  ) : catalog.kind === "loading" ? (
    <section className="op-framework-message" role="status">
      正在读取对象目录…
    </section>
  ) : catalog.kind === "error" ? (
    <section className="op-framework-message" role="alert">
      <h2>无法读取对象目录</h2>
      <p>{catalog.message}</p>
    </section>
  ) : null;
  return (
    <div className="op-shell op-embedded">
      <div className="op-workspace">
        <main className="op-main">
          {page === "objects" && (
            <ObjectToolbarControls
              filter={filter}
              setFilter={setFilter}
              search={search}
              setSearch={setSearch}
              tags={tags}
              setTags={setTags}
              thumbnailSize={thumbnailSize}
              setThumbnailSize={setThumbnailSize}
              availableTags={availableTags}
              mode={null}
              setMode={() => {}}
              canAnnotate={false}
              generateObjects={
                !loading && project
                  ? () => setGenerationTarget(project.id)
                  : undefined
              }
              unavailableReason="请在对象版本或尝试的场景预览中选择画面"
              importObjects={
                !loading && project
                  ? () => setImportTarget(project.id)
                  : undefined
              }
            />
          )}
          <div className="op-main-content op-framework-content">
            <div className="op-framework-body">
              {message}
              {page === "objects" && catalog.kind === "ready" && (
                <div className="op-library-scroll">
                  <ObjectCatalogGrid
                    objects={catalog.objects}
                    filter={filter}
                    search={search}
                    tags={tags}
                    thumbnailSize={thumbnailSize}
                    selected={selectedObject}
                    select={(id) => {
                      selectObject(id);
                      setNotice(`已选择对象 ${id}`);
                    }}
                  />
                  {selectedObjectRecord && project && (
                    <ObjectCatalogDetails
                      object={selectedObjectRecord}
                      objects={catalog.objects}
                      currentProjectId={project.id}
                      select={selectObject}
                    />
                  )}
                </div>
              )}
              {project && !loading && (page === "manufacture" || tasksOpen) && (
                <div className="op-library-scroll">
                  {selectedObject && (
                    <button
                      onClick={() => {
                        setSelectedObject(undefined);
                        setSelectedTask(undefined);
                      }}
                    >
                      查看全部对象迭代
                    </button>
                  )}
                  <ObjectProductionTasks
                    query={taskQuery}
                    objectId={selectedObject}
                    taskId={selectedTask}
                    manufacture={page === "manufacture"}
                    refresh={() => {
                      void refreshTasks();
                    }}
                    select={(task) => {
                      setSelectedObject(task.objectId!);
                      setSelectedTask(task.id);
                      navigate("manufacture");
                    }}
                    openObject={(id) => {
                      setSearch("");
                      setFilter("全部");
                      setTags([]);
                      setSelectedObject(id);
                      navigate("objects");
                    }}
                    execute={(task) => {
                      if (taskQuery.kind === "ready")
                        setExecution(
                          new ObjectTaskExecution(
                            project.id,
                            taskQuery.snapshot,
                            task.id,
                            call,
                            async () => {
                              setCatalogRevision((value) => value + 1);
                              return refreshTasks();
                            },
                          ),
                        );
                    }}
                  />
                </div>
              )}
              {notice && <p role="status">{notice}</p>}
              {page === "objects" && !selectedObject && (
                <div className="op-framework-contents" aria-label="子对象内容">
                  <span>选择对象后查看组件、文件、引用和版本</span>
                </div>
              )}
            </div>
            <aside
              className="op-framework-rail"
              aria-label={page === "objects" ? "对象任务" : "制造任务"}
            >
              <button
                disabled={page === "manufacture" || !project || loading}
                aria-label="展开任务"
                aria-expanded={page === "manufacture" || tasksOpen}
                onClick={() => setTasksOpen((value) => !value)}
                title={
                  taskQuery.kind === "loading"
                    ? "正在读取对象任务"
                    : taskQuery.kind === "error"
                      ? `无法读取对象任务：${taskQuery.message}`
                      : taskCount
                        ? `${taskCount} 项对象任务`
                        : "当前项目尚无对象任务"
                }
              >
                <Icon name="back" />
              </button>
              <button
                disabled={!selectedObject}
                aria-label="返回所选对象"
                title="返回所选对象"
                onClick={() => {
                  setSearch("");
                  setFilter("全部");
                  setTags([]);
                  navigate("objects");
                }}
              >
                <Icon name="features" />
              </button>
            </aside>
          </div>
          <footer className="op-statusbar" aria-label="提示区域" />
          {execution && (
            <ObjectTaskExecutionDialog
              session={execution}
              initialCheck={
                initialCheck?.target.taskId === execution.task.id
                  ? initialCheck
                  : undefined
              }
              close={() => setExecution(null)}
            />
          )}
          {importTarget && (
            <ObjectImportDialog
              projectId={importTarget}
              projects={projects}
              close={() => setImportTarget(undefined)}
              notify={setNotice}
              openObject={(targetId, objectId) => {
                if (targetId !== project?.id) {
                  setNotice(`请切换到目标项目 ${targetId} 后打开导入对象。`);
                  return;
                }
                setSearch("");
                setFilter("全部");
                setTags([]);
                selectObject(objectId);
                setCatalogRevision((value) => value + 1);
                setImportTarget(undefined);
                setNotice("已导入，待验证。");
              }}
            />
          )}
          {generationTarget && (
            <ObjectGenerateDialog
              key={generationTarget}
              projectId={generationTarget}
              close={() => setGenerationTarget(undefined)}
              openManufacture={(targetId, objectId, taskId) => {
                if (targetId !== project?.id) {
                  setNotice(`请切换到目标项目 ${targetId} 后打开制造任务。`);
                  return;
                }
                setSearch("");
                setFilter("全部");
                setTags([]);
                selectObject(objectId);
                setSelectedTask(taskId);
                setCatalogRevision((value) => value + 1);
                void refreshTasks();
                setGenerationTarget(undefined);
                setNotice("已定位新建制造任务；尚未入队或执行。");
                navigate("manufacture");
              }}
              openObject={(targetId, objectId) => {
                if (targetId !== project?.id) {
                  setNotice(`请切换到目标项目 ${targetId} 后打开生成对象。`);
                  return;
                }
                setSearch("");
                setFilter("全部");
                setTags([]);
                selectObject(objectId);
                setCatalogRevision((value) => value + 1);
                setGenerationTarget(undefined);
                void refreshTasks();
                setNotice("对象和待调度任务已创建，尚未生成或验收。");
              }}
            />
          )}
        </main>
      </div>
    </div>
  );
}
