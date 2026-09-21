import { useRef, useState } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import { call } from "../api";

type ExternalObject = {
  id: string;
  name: string;
  files?: Array<{ path: string }>;
  versions?: Array<{ versionId: string }>;
};

export function ObjectImportDialog({
  close,
  notify,
  projectId,
}: {
  close: () => void;
  notify: (message: string) => void;
  projectId?: string;
}) {
  const [path, setPath] = useState("");
  const [name, setName] = useState("");
  const [files, setFiles] = useState<string[]>([]);
  const [sourceProjectId, setSourceProjectId] = useState("");
  const [objectId, setObjectId] = useState("");
  const [objects, setObjects] = useState<ExternalObject[]>([]);
  const [baseline, setBaseline] = useState<"latestAccepted" | string>(
    "latestAccepted",
  );
  const [preparationId, setPreparationId] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const filePicker = useRef<HTMLInputElement>(null);
  const folderPicker = useRef<HTMLInputElement>(null);
  const selected = objects.find((object) => object.id === objectId);
  const contents = selected?.files?.map((file) => file.path) ?? files;
  const chooseResources = (resources: FileList | null) => {
    if (!resources?.length) return;
    const first = resources[0]!;
    const folder = first.webkitRelativePath.split("/").slice(0, -1)[0];
    setFiles(
      Array.from(resources, (file) => file.webkitRelativePath || file.name),
    );
    setPath(folder || `${resources.length} 个文件 · ${first.name}`);
    setName(folder || first.name.replace(/\.[^.]+$/, ""));
  };
  const inspect = async () => {
    if (!path.trim() || !sourceProjectId.trim()) return;
    setBusy(true);
    setError("");
    try {
      const result = await call<{ objects: ExternalObject[] }>(
        "object.inspectExternal",
        { path: path.trim(), projectId: sourceProjectId.trim() },
      );
      setObjects(result.objects);
      const first = result.objects[0];
      if (first) {
        setObjectId(first.id);
        setName(first.name);
        setBaseline("latestAccepted");
      }
      if (!first) setObjectId("");
    } catch (error) {
      setObjects([]);
      setObjectId("");
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog title="导入对象" close={close} className="op-object-import-dialog">
      <p className="op-muted">选择外部文件或文件夹，将关联资源组成一个对象。</p>
      <form
        onSubmit={async (event) => {
          event.preventDefault();
          if (!path.trim() || !name.trim() || !objectId.trim()) return;
          if (!projectId) {
            notify("请选择一个已登记项目后再准备对象导入");
            return;
          }
          setBusy(true);
          setError("");
          try {
            const result = await call<{ preparationId: string }>(
              "object.prepareImport",
              {
                targetProjectId: projectId,
                source: {
                  path: path.trim(),
                  projectId: sourceProjectId.trim(),
                },
                objectId: objectId.trim(),
                baseline:
                  baseline === "latestAccepted"
                    ? { kind: "latestAccepted" }
                    : { kind: "pinnedVersion", versionId: baseline },
              },
            );
            setPreparationId(result.preparationId);
            notify(`已准备对象「${name.trim()}」，等待正式提交`);
          } catch (error) {
            setError(error instanceof Error ? error.message : String(error));
          } finally {
            setBusy(false);
          }
        }}
      >
        <label className="op-field">
          资源来源
          <input
            aria-label="资源路径"
            value={path}
            onChange={(event) => {
              setPath(event.target.value);
              setFiles([]);
            }}
            placeholder="输入外部资源路径…"
          />
        </label>
        <div className="op-object-form-row">
          <label className="op-field">
            源项目 ID
            <input
              aria-label="源项目 ID"
              value={sourceProjectId}
              onChange={(event) => setSourceProjectId(event.target.value)}
              placeholder="已登记项目的 ID"
            />
          </label>
          <label className="op-field">
            对象 ID
            <input
              aria-label="源对象 ID"
              value={objectId}
              onChange={(event) => setObjectId(event.target.value)}
              placeholder="要准备的对象 ID"
            />
          </label>
        </div>
        <div className="op-resource-picker">
          <button type="button" onClick={() => filePicker.current?.click()}>
            <Icon name="project" />
            选择文件
          </button>
          <button type="button" onClick={() => folderPicker.current?.click()}>
            <Icon name="folder" />
            选择文件夹
          </button>
          <input
            hidden
            type="file"
            multiple
            ref={filePicker}
            onChange={(event) => {
              chooseResources(event.target.files);
              event.target.value = "";
            }}
          />
          <input
            hidden
            type="file"
            multiple
            ref={(node) => {
              folderPicker.current = node;
              node?.setAttribute("webkitdirectory", "");
            }}
            onChange={(event) => {
              chooseResources(event.target.files);
              event.target.value = "";
            }}
          />
        </div>
        <button
          type="button"
          onClick={inspect}
          disabled={busy || !path.trim() || !sourceProjectId.trim()}
        >
          读取源对象
        </button>
        {objects.length > 0 && (
          <fieldset className="op-import-folders">
            <legend>源对象目录</legend>
            {objects.map((object) => (
              <label key={object.id}>
                <input
                  type="radio"
                  name="source-object"
                  checked={objectId === object.id}
                  onChange={() => {
                    setObjectId(object.id);
                    setName(object.name);
                    setBaseline("latestAccepted");
                  }}
                />
                <Icon name="folder" />
                <span>
                  {object.name} ({object.id})
                </span>
              </label>
            ))}
          </fieldset>
        )}
        <div className="op-import-contents">
          {contents.length ? (
            contents.map((name) => (
              <span key={name}>
                <Icon name="project" />
                {name}
              </span>
            ))
          ) : (
            <p className="op-muted">先读取源对象目录以显示真实文件清单。</p>
          )}
        </div>
        <div className="op-object-form-row">
          <label className="op-field">
            对象名称
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              required
            />
          </label>
          <label className="op-field">
            接受版本
            <select
              value={baseline}
              onChange={(event) => setBaseline(event.target.value)}
              disabled={!selected}
            >
              <option value="latestAccepted">最新接受版本</option>
              {selected?.versions?.map((version) => (
                <option key={version.versionId} value={version.versionId}>
                  固定版本 {version.versionId}
                </option>
              ))}
            </select>
          </label>
          <label className="op-field">
            对象类型
            <select defaultValue="模型">
              {["图像", "音频", "模型", "场景", "翻译", "脚本", "其他"].map(
                (type) => (
                  <option key={type}>{type}</option>
                ),
              )}
            </select>
          </label>
        </div>
        <label className="op-field">
          标签
          <input placeholder="添加标签，使用逗号分隔" />
        </label>
        <label className="op-field">
          补充说明
          <textarea
            rows={2}
            placeholder="描述资源用途、素材来源或导入时需要注意的事项…"
          />
        </label>
        <div className="op-dialog-note">
          <Icon name="review" />
          <span>
            只读准备会校验源项目对象引用和文件哈希，不修改目标对象目录；正式提交仍由后续流程负责。
          </span>
        </div>
        <footer>
          <button type="button" onClick={close}>
            取消
          </button>
          <button
            type="submit"
            className="primary"
            disabled={
              busy ||
              !path.trim() ||
              !name.trim() ||
              !projectId ||
              !sourceProjectId.trim() ||
              !objectId.trim() ||
              objects.length === 0
            }
          >
            <Icon name="import" />
            导入对象
          </button>
        </footer>
        {preparationId && (
          <p role="status">只读准备完成：{preparationId}。正式导入尚未提交。</p>
        )}
        {error && <p role="alert">导入准备失败：{error}</p>}
      </form>
    </Dialog>
  );
}
