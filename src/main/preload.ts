import { contextBridge, ipcRenderer } from "electron";
import type { WindowCommand, WindowState } from "../shared/window";
contextBridge.exposeInMainWorld("beaver", {
  windowControl: (command: WindowCommand): Promise<WindowState> =>
    ipcRenderer.invoke("beaver:window", command),
  onWindowState: (listener: (state: WindowState) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, state: WindowState) =>
      listener(state);
    ipcRenderer.on("beaver:window-state", handler);
    return () => ipcRenderer.removeListener("beaver:window-state", handler);
  },
  call: async (method: string, input?: unknown) => {
    const response: unknown = await ipcRenderer.invoke("beaver:call", {
      method,
      input,
    });
    const r = response as { ok: boolean; value?: unknown; error?: string };
    if (!r.ok) throw new Error(r.error || "操作失败");
    return r.value;
  },
  subscribe: (listener: () => void) => {
    const handler = () => listener();
    ipcRenderer.on("beaver:changed", handler);
    return () => ipcRenderer.removeListener("beaver:changed", handler);
  },
});
