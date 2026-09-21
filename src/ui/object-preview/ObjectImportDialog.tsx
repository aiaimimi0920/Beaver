import { useRef, useState } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";

const folders = [
  {
    name: "澪-NPR角色",
    path: "D:\\创作素材\\澪-NPR角色",
    contents: [
      "角色模型.blend",
      "基础色贴图.png",
      "法线贴图.png",
      "NPR材质.gdshader",
    ],
  },
  {
    name: "午后教室",
    path: "D:\\创作素材\\午后教室",
    contents: ["教室.tscn", "课桌.glb", "环境材质.tres"],
  },
  {
    name: "木纹贴图",
    path: "D:\\创作素材\\木纹贴图",
    contents: ["木纹基础色.png", "表面法线.png"],
  },
];

export function ObjectImportDialog({
  close,
  notify,
}: {
  close: () => void;
  notify: (message: string) => void;
}) {
  const [path, setPath] = useState(folders[0]!.path);
  const [name, setName] = useState(folders[0]!.name);
  const [files, setFiles] = useState<string[]>([]);
  const filePicker = useRef<HTMLInputElement>(null);
  const folderPicker = useRef<HTMLInputElement>(null);
  const selected = folders.find((folder) => folder.path === path);
  const contents = selected?.contents ?? files;
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
  return (
    <Dialog title="导入对象" close={close} className="op-object-import-dialog">
      <p className="op-muted">选择外部文件或文件夹，将关联资源组成一个对象。</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (!path.trim() || !name.trim()) return;
          notify(
            `已预览导入「${name.trim()}」 · 测试操作，未读取文件内容或创建真实对象`,
          );
          close();
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
        <fieldset className="op-import-folders">
          <legend>示例资源</legend>
          {folders.map((folder) => (
            <label key={folder.path}>
              <input
                type="radio"
                name="object-folder"
                checked={path === folder.path}
                onChange={() => {
                  setPath(folder.path);
                  setName(folder.name);
                  setFiles([]);
                }}
              />
              <Icon name="folder" />
              <span>{folder.name}</span>
            </label>
          ))}
        </fieldset>
        <div className="op-import-contents">
          {contents.length ? (
            contents.map((name) => (
              <span key={name}>
                <Icon name="project" />
                {name}
              </span>
            ))
          ) : (
            <p className="op-muted">自定义路径仅用于界面预览。</p>
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
            UI 预览：仅展示资源名称和填写信息，不读取文件内容、不导入项目。
          </span>
        </div>
        <footer>
          <button type="button" onClick={close}>
            取消
          </button>
          <button
            type="submit"
            className="primary"
            disabled={!path.trim() || !name.trim()}
          >
            <Icon name="import" />
            导入对象
          </button>
        </footer>
      </form>
    </Dialog>
  );
}
