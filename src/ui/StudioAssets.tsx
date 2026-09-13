import { useState } from "react";
import { Dialog, Field } from "./components";
import { Icon, type IconName } from "./Icon";
import type { StudioAsset } from "./studio-preview";

export function AssetThumbnail({ asset }: { asset: StudioAsset }) {
  const icons: Record<StudioAsset["type"], IconName> = {
    图片: "assets",
    音频: "music",
    模型: "features",
    场景: "overview",
    翻译: "translation",
  };
  return (
    <div
      className={`studio-thumbnail ${asset.folder.startsWith("人物") ? "portrait" : ""}`}
      aria-hidden="true"
    >
      {asset.folder.startsWith("人物") ? (
        <>
          <i />
          <b />
        </>
      ) : (
        <Icon name={icons[asset.type]} />
      )}
    </div>
  );
}
export function StudioAssets({
  assets,
  change,
  request,
  openFiles,
}: {
  assets: StudioAsset[];
  change: (assets: StudioAsset[]) => void;
  request: (goal: string, refs: string[]) => void;
  openFiles?: () => void;
}) {
  const [folder, setFolder] = useState("全部素材");
  const [selected, setSelected] = useState(["cheng"]);
  const [search, setSearch] = useState("");
  const [type, setType] = useState("全部");
  const [sort, setSort] = useState("最近添加");
  const [zoom, setZoom] = useState(140);
  const [dialog, setDialog] = useState<
    "preview" | "generate" | "modify" | "style" | null
  >(null);
  const [refs, setRefs] = useState<string[]>([]);
  const [prompt, setPrompt] = useState("");
  const [generateType, setGenerateType] = useState("图片");
  const [count, setCount] = useState(1);
  const [destination, setDestination] = useState("人物");
  const [template, setTemplate] = useState("");
  const picked = assets.filter((a) => selected.includes(a.id));
  const current = picked[0];
  const matchesFolder = (a: StudioAsset, f: string) =>
    f === "全部素材" ||
    (f === "收藏"
      ? a.favorite
      : f === "未分类"
        ? !a.folder
        : a.folder === f || a.folder.startsWith(`${f}/`));
  const visible = assets.filter(
    (a) =>
      matchesFolder(a, folder) &&
      (type === "全部" || a.type === type) &&
      `${a.name} ${a.tags.join(" ")}`.includes(search),
  );
  if (sort === "名称")
    visible.sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const folders = [
    ...new Set([
      "全部素材",
      "收藏",
      "未分类",
      ...assets.flatMap((a) => [a.folder.split("/")[0]!, a.folder]),
    ]),
  ];
  const update = (patch: Partial<StudioAsset>) =>
    change(assets.map((a) => (a.id === current?.id ? { ...a, ...patch } : a)));
  const toggle = (id: string) =>
    setSelected((old) =>
      old.includes(id) ? old.filter((x) => x !== id) : [...old, id],
    );
  const generate = (references: string[]) => {
    setRefs(references);
    setPrompt("");
    setTemplate("");
    setCount(1);
    setDialog("generate");
  };
  return (
    <div className="studio-assets">
      <aside className="studio-directory">
        <div className="studio-toolbar">
          <strong>素材库</strong>
        </div>
        <div className="studio-scroll">
          {folders.map((f) => (
            <button
              key={f}
              className={`studio-entry ${folder === f ? "active" : ""} ${f.includes("/") ? "nested" : ""}`}
              onClick={() => {
                setFolder(f);
                setSelected([]);
              }}
            >
              <Icon name="folder" />
              <span>{f.split("/").pop()}</span>
              <small>{assets.filter((a) => matchesFolder(a, f)).length}</small>
            </button>
          ))}
        </div>
        {openFiles && (
          <button className="studio-existing" onClick={openFiles}>
            项目文件 ↗
          </button>
        )}
      </aside>
      <section className="studio-gallery">
        <div className="studio-toolbar">
          <input
            aria-label="搜索素材"
            placeholder="搜索素材"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <button className="primary" onClick={() => generate([])}>
            生成
          </button>
        </div>
        <div className="studio-toolbar">
          <select
            aria-label="素材类型"
            value={type}
            onChange={(e) => setType(e.target.value)}
          >
            {["全部", "图片", "音频", "模型", "场景", "翻译"].map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
          <select
            aria-label="素材排序"
            value={sort}
            onChange={(e) => setSort(e.target.value)}
          >
            <option>最近添加</option>
            <option>名称</option>
          </select>
          <div className="studio-spacer" />
          <input
            type="range"
            aria-label="缩略图大小"
            min={105}
            max={210}
            value={zoom}
            onChange={(e) => setZoom(Number(e.target.value))}
          />
        </div>
        <div
          className="studio-gallery-grid studio-scroll"
          style={{
            gridTemplateColumns: `repeat(auto-fill, minmax(min(100%, ${zoom}px), 1fr))`,
          }}
        >
          {visible.map((a) => (
            <div
              key={a.id}
              className={`studio-asset-card ${selected.includes(a.id) ? "active" : ""}`}
            >
              <button
                aria-label={`选择${a.name}`}
                onClick={(e) =>
                  e.ctrlKey || e.metaKey ? toggle(a.id) : setSelected([a.id])
                }
                onDoubleClick={() => {
                  setSelected([a.id]);
                  setDialog("preview");
                }}
                onKeyDown={(e) => {
                  if (e.code === "Space") {
                    e.preventDefault();
                    setSelected([a.id]);
                    setDialog("preview");
                  }
                }}
              >
                <AssetThumbnail asset={a} />
                <span>{a.name}</span>
                <small>{a.type}</small>
              </button>
              <input
                aria-label={`多选${a.name}`}
                type="checkbox"
                checked={selected.includes(a.id)}
                onChange={() => toggle(a.id)}
              />
            </div>
          ))}
          {visible.length === 0 && <p className="studio-empty">无匹配素材</p>}
        </div>
        <footer className="studio-status">
          <span>{visible.length} 项</span>
          <span>已选 {picked.length} 项</span>
        </footer>
      </section>
      <aside className="studio-inspector">
        <div className="studio-toolbar">
          <strong>
            {picked.length > 1 ? `已选 ${picked.length} 项` : "属性"}
          </strong>
          {current && (
            <button onClick={() => setDialog("preview")}>预览</button>
          )}
        </div>
        {current ? (
          <>
            <div className="studio-scroll studio-properties">
              {picked.length === 1 ? (
                <>
                  <AssetThumbnail asset={current} />
                  <input
                    aria-label="素材名称"
                    value={current.name}
                    onChange={(e) => update({ name: e.target.value })}
                  />
                  <label>
                    <input
                      type="checkbox"
                      checked={current.favorite}
                      onChange={(e) => update({ favorite: e.target.checked })}
                    />{" "}
                    收藏
                  </label>
                </>
              ) : (
                <div className="studio-mini-grid">
                  {picked.map((a) => (
                    <div key={a.id}>
                      <AssetThumbnail asset={a} />
                      <small>{a.name}</small>
                    </div>
                  ))}
                </div>
              )}
              <section>
                <small>标签</small>
                <div className="studio-chips">
                  {[...new Set(picked.flatMap((a) => a.tags))].map((tag) => (
                    <button
                      key={tag}
                      onClick={() => {
                        setSearch(tag);
                        setFolder("全部素材");
                      }}
                    >
                      {tag}
                    </button>
                  ))}
                </div>
              </section>
              <section>
                <small>文件夹</small>
                <p>{[...new Set(picked.map((a) => a.folder))].join(" · ")}</p>
              </section>
              {picked.length === 1 && (
                <Field label="备注">
                  <textarea
                    value={current.notes}
                    onChange={(e) => update({ notes: e.target.value })}
                  />
                </Field>
              )}
            </div>
            <div className="studio-property-actions">
              <button
                onClick={() => {
                  setPrompt("");
                  setDialog("modify");
                }}
              >
                AI 修改
              </button>
              <button onClick={() => generate(picked.map((a) => a.name))}>
                参考生成
              </button>
              {picked.length > 1 && (
                <button
                  onClick={() => {
                    setPrompt("统一描边、比例与配色，保留原有内容。");
                    setDialog("style");
                  }}
                >
                  统一风格
                </button>
              )}
            </div>
          </>
        ) : (
          <p className="studio-empty">未选择素材</p>
        )}
      </aside>
      {dialog === "preview" && (
        <Dialog
          title={picked.length > 1 ? "对比素材" : (current?.name ?? "预览")}
          close={() => setDialog(null)}
        >
          <div className="studio-comparison">
            {picked.map((a) => (
              <section key={a.id}>
                <AssetThumbnail asset={a} />
                <h3>{a.name}</h3>
                <small>示例占位</small>
              </section>
            ))}
          </div>
          <footer>
            <button onClick={() => setDialog(null)}>关闭</button>
            <button
              className="primary"
              onClick={() => {
                setPrompt("");
                setDialog("modify");
              }}
            >
              AI 修改
            </button>
          </footer>
        </Dialog>
      )}
      {dialog && dialog !== "preview" && (
        <Dialog
          title={
            dialog === "generate"
              ? "生成素材"
              : dialog === "style"
                ? "统一风格"
                : "AI 修改素材"
          }
          close={() => setDialog(null)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!prompt.trim()) return;
              request(
                dialog === "generate"
                  ? `生成 ${count} 份${generateType}：${prompt}；归档到${destination}。`
                  : prompt,
                dialog === "generate" ? refs : picked.map((a) => a.name),
              );
              setDialog(null);
            }}
          >
            {dialog === "generate" && (
              <>
                <div className="studio-form-grid">
                  <Field label="类型">
                    <select
                      value={generateType}
                      onChange={(e) => setGenerateType(e.target.value)}
                    >
                      {["图片", "音频", "模型", "场景", "翻译"].map((t) => (
                        <option key={t}>{t}</option>
                      ))}
                    </select>
                  </Field>
                  <Field label="数量">
                    <input
                      type="number"
                      min={1}
                      max={8}
                      value={count}
                      onChange={(e) => setCount(Number(e.target.value))}
                    />
                  </Field>
                </div>
                <Field label="提示词模板">
                  <select
                    value={template}
                    onChange={(e) => {
                      const key = e.target.value;
                      setTemplate(key);
                      if (key) {
                        setGenerateType(key === "music" ? "音频" : "图片");
                        setPrompt(
                          key === "music"
                            ? "生成适合雨夜酒吧的舒缓循环音乐。"
                            : "依据角色视觉规范，生成低饱和像素风立绘，透明背景。",
                        );
                      }
                    }}
                  >
                    <option value="">自定义</option>
                    <option value="portrait">角色立绘</option>
                    <option value="music">环境音乐</option>
                  </select>
                </Field>
              </>
            )}
            <Field label="要求">
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="希望得到什么效果？"
              />
            </Field>
            {dialog === "generate" && (
              <Field label="保存到">
                <select
                  value={destination}
                  onChange={(e) => setDestination(e.target.value)}
                >
                  {["人物", "场景", "声音", "参考", "本地化"].map((f) => (
                    <option key={f}>{f}</option>
                  ))}
                </select>
              </Field>
            )}
            <div className="studio-chips">
              {(dialog === "generate" ? refs : picked.map((a) => a.name)).map(
                (ref, i) => (
                  <span className="studio-chip" key={i}>
                    {ref}
                  </span>
                ),
              )}
            </div>
            <footer>
              <button type="button" onClick={() => setDialog(null)}>
                取消
              </button>
              <button
                className="primary"
                disabled={
                  !prompt.trim() ||
                  (dialog === "generate" &&
                    (!Number.isInteger(count) || count < 1 || count > 8))
                }
              >
                {dialog === "generate" ? "生成" : "提交修改"}
              </button>
            </footer>
          </form>
        </Dialog>
      )}
    </div>
  );
}
