import { useState } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";

export function ObjectGenerateDialog({
  close,
  notify,
}: {
  close: () => void;
  notify: (message: string) => void;
}) {
  const [type, setType] = useState("模型");
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  return (
    <Dialog
      title="生成对象"
      close={close}
      className="op-object-generate-dialog"
    >
      <p className="op-muted">描述想要的对象，以生成任务的方式开始创作。</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (!prompt.trim()) return;
          notify(
            `已预览添加${type}生成任务「${name.trim() || prompt.trim()}」 · 未创建真实任务`,
          );
          close();
        }}
      >
        <div className="op-object-form-row">
          <label className="op-field">
            对象名称（选填）
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="例如：澪 · NPR 角色"
            />
          </label>
          <label className="op-field">
            生成内容
            <select
              value={type}
              onChange={(event) => setType(event.target.value)}
            >
              {["图像", "音频", "模型", "场景", "翻译", "脚本", "其他"].map(
                (value) => (
                  <option key={value}>{value}</option>
                ),
              )}
            </select>
          </label>
        </div>
        <label className="op-field">
          任务描述
          <textarea
            rows={4}
            required
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            placeholder="描述对象的外观、风格和用途，例如：一个青蓝短发、穿学院制服的二次元 NPR 角色…"
          />
        </label>
        <label className="op-field">
          验收要求（选填）
          <textarea
            rows={2}
            placeholder="例如：包含模型和贴图，可以直接用于 Godot 场景。"
          />
        </label>
        <div className="op-dialog-note">
          <Icon name="tasks" />
          <span>UI 预览：此入口用于添加对象生成任务，确认后仅显示反馈。</span>
        </div>
        <footer>
          <button type="button" onClick={close}>
            取消
          </button>
          <button type="submit" className="primary" disabled={!prompt.trim()}>
            <Icon name="add" />
            添加任务
          </button>
        </footer>
      </form>
    </Dialog>
  );
}
