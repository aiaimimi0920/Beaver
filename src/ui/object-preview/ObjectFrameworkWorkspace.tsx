import { useState } from "react";
import type { Project } from "../../shared/types";
import { Icon } from "../Icon";
import { ObjectToolbarControls } from "./ObjectToolbarControls";
import { ObjectImportDialog } from "./ObjectImportDialog";
import type { ObjectType } from "./object-categories";
import { ObjectCatalogGrid } from "./ObjectCatalogGrid";
import { useObjectCatalog } from "./use-object-catalog";

export function ObjectFrameworkWorkspace({
  project,
  loading,
  page,
  createProject,
}: {
  project?: Project;
  loading: boolean;
  page: "objects" | "manufacture";
  createProject: () => void;
}) {
  const [filter, setFilter] = useState<ObjectType | "全部">("全部");
  const [search, setSearch] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [thumbnailSize, setThumbnailSize] = useState(240);
  const [notice, setNotice] = useState("");
  const [selectedObject, setSelectedObject] = useState<string>();
  const [importTarget, setImportTarget] = useState<string>();
  const catalog = useObjectCatalog(project?.id, search);
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
              availableTags={[]}
              mode={null}
              setMode={() => {}}
              canAnnotate={false}
              unavailableReason="对象制作能力尚未接通"
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
                    thumbnailSize={thumbnailSize}
                    selected={selectedObject}
                    select={(id) => {
                      setSelectedObject(id);
                      setNotice(`已选择对象 ${id}`);
                    }}
                  />
                </div>
              )}
              {notice && <p role="status">{notice}</p>}
              {page === "objects" && (
                <div className="op-framework-contents" aria-label="子对象内容">
                  <button disabled aria-label="展开子对象">
                    <span className="op-framework-up">
                      <Icon name="chevronDown" />
                    </span>
                  </button>
                  <span>尚未选择对象</span>
                </div>
              )}
            </div>
            <aside
              className="op-framework-rail"
              aria-label={page === "objects" ? "对象任务" : "制造任务"}
            >
              <button disabled aria-label="展开任务" title="尚无可读取的任务">
                <Icon name="back" />
              </button>
              <button
                disabled
                aria-label="未绑定对象"
                title="选择对象后查看对应任务"
              >
                <Icon name="features" />
              </button>
            </aside>
          </div>
          <footer className="op-statusbar" aria-label="提示区域" />
          {importTarget && (
            <ObjectImportDialog
              projectId={importTarget}
              close={() => setImportTarget(undefined)}
              notify={setNotice}
            />
          )}
        </main>
      </div>
    </div>
  );
}
