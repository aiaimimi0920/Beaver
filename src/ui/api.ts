import type { Project, Task, Settings, Feature } from "../shared/types";
import type { WindowCommand, WindowState } from "../shared/window";
declare global {
  interface Window {
    beaver: {
      windowControl(command: WindowCommand): Promise<WindowState>;
      onWindowState(listener: (state: WindowState) => void): () => void;
      call(method: string, input?: unknown): Promise<unknown>;
      subscribe(listener: () => void): () => void;
    };
  }
}
export interface State {
  projects: Project[];
  tasks: Task[];
  settings: Settings;
  features: Feature[];
}
export const call = async <T>(method: string, input?: unknown): Promise<T> =>
  window.beaver.call(method, input) as Promise<T>;
export type Run = (
  work: () => Promise<unknown>,
  message?: string,
) => Promise<void>;
export const assetUrl = (id: string, file: string, revision = "") =>
  `${"__TAURI__" in window ? "http://beaver-asset.localhost/" : "beaver-asset://"}${id}/${file.split("/").map(encodeURIComponent).join("/")}${revision ? `?v=${encodeURIComponent(revision)}` : ""}`;
export const statusNames: Record<Task["status"], string> = {
  queued: "等待执行",
  running: "执行中",
  waitingChildren: "子任务进行中",
  awaitingInput: "需要你补充",
  interrupted: "已中止 · 可继续",
  failed: "执行失败",
  conflict: "合入冲突",
  completed: "已合入",
  rolledBack: "已回退",
};
export function size(bytes: number): string {
  return bytes < 1024
    ? `${bytes} B`
    : bytes < 1048576
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / 1048576).toFixed(1)} MB`;
}
