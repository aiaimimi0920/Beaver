import type { WindowState } from "../shared/window";
import { nativeTitlebarCommand } from "./native-titlebar";

interface NativeApi {
  core: {
    invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  };
  event: {
    listen<T>(
      name: string,
      listener: (event: { payload: T }) => void,
    ): Promise<() => void>;
  };
}

const native = (window as Window & { __TAURI__?: NativeApi }).__TAURI__;
if (native && !window.beaver) {
  document.addEventListener("mousedown", (event) => {
    if (event.defaultPrevented || !(event.target instanceof Element)) return;
    const command = nativeTitlebarCommand(
      event.button,
      event.detail,
      Boolean(event.target.closest("[data-native-drag-region]")),
      Boolean(
        event.target.closest(
          "button, a, input, select, textarea, [role='button'], [contenteditable='true']",
        ),
      ),
    );
    if (!command) return;
    event.preventDefault();
    void native.core
      .invoke("beaver_window", { command })
      .catch((error) =>
        console.error("Native titlebar operation failed", error),
      );
  });
  const listen = <T>(event: string, listener: (value: T) => void) => {
    let disposed = false;
    let stop: (() => void) | undefined;
    void native.event
      .listen<T>(event, ({ payload }) => {
        if (!disposed) listener(payload);
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error) =>
        console.error("Native event subscription failed", error),
      );
    return () => {
      disposed = true;
      stop?.();
    };
  };
  window.beaver = {
    call: (method, input) =>
      native.core.invoke("beaver_call", { method, input: input ?? null }),
    windowControl: (command) =>
      native.core.invoke<WindowState>("beaver_window", { command }),
    onWindowState: (listener) =>
      listen<WindowState>("beaver:window-state", listener),
    subscribe: (listener) => listen("beaver:changed", listener),
  };
}
