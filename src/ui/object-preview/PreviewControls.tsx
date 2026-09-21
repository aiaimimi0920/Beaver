import { useState, type ReactNode } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import { statusTone, type DemoStatus } from "./mock-objects";

export type DemoAction = (title: string, context?: string) => void;

export function Status({ value }: { value: DemoStatus }) {
  return (
    <span className={`op-status op-${statusTone(value)}`}>
      <i />
      {value}
    </span>
  );
}

export function PreviewPrompt({
  label,
  placeholder,
  context,
  notify,
  children,
}: {
  label: string;
  placeholder: string;
  context: string;
  notify: (message: string) => void;
  children?: ReactNode;
}) {
  const [value, setValue] = useState("");
  return (
    <form
      className="op-prompt"
      onSubmit={(event) => {
        event.preventDefault();
        if (!value.trim()) return;
        notify(`已演示提交「${value.trim()}」 · ${context} · 未创建真实任务`);
        setValue("");
      }}
    >
      {children}
      <textarea
        aria-label={label}
        placeholder={placeholder}
        value={value}
        onChange={(event) => setValue(event.target.value)}
        rows={2}
      />
      <footer>
        <span>{context}</span>
        <button
          type="submit"
          disabled={!value.trim()}
          aria-label={`提交${label}`}
        >
          <Icon name="add" />
          提交修改
        </button>
      </footer>
    </form>
  );
}

export function DemoActionDialog({
  title,
  context,
  close,
  notify,
}: {
  title: string;
  context: string;
  close: () => void;
  notify: (message: string) => void;
}) {
  return (
    <Dialog title={title} close={close} className="op-action-dialog">
      <p className="op-muted">测试数据 · {context || "当前演示项目"}</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          notify(`已演示「${title}」 · 仅展示操作反馈，未写入项目`);
          close();
        }}
      >
        <label className="op-field">
          目标与调整要求
          <textarea
            rows={4}
            defaultValue={
              title === "调整任务拆分"
                ? "保留角色与教室两个独立对象；两者验收后，再开始舞蹈演出整合。"
                : ""
            }
            placeholder="写下你希望调整的内容…"
          />
        </label>
        <label className="op-field">
          验收要求
          <textarea rows={2} placeholder="什么样的结果可以进入下一步？" />
        </label>
        <div className="op-dialog-note">
          <Icon name="review" />
          <span>这次只预览界面。不会启动 Codex、调用插件或修改对象。</span>
        </div>
        <footer>
          <button type="button" onClick={close}>
            取消
          </button>
          <button className="primary" type="submit">
            确认 · 演示操作
          </button>
        </footer>
      </form>
    </Dialog>
  );
}
