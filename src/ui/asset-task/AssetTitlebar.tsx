import { useEffect, useState } from "react";
import type { WindowCommand } from "../../shared/window";
import { Icon } from "../Icon";
import { useNotify } from "../Notifications";
import { errorMessage } from "../notification-state";

export function AssetTitlebar({ title }: { title: string }) {
  const [maximized, setMaximized] = useState(false);
  const notify = useNotify();
  useEffect(() => {
    const unsubscribe = window.beaver.onWindowState((state) =>
      setMaximized(state.maximized),
    );
    void window.beaver
      .windowControl("state")
      .then((state) => setMaximized(state.maximized))
      .catch((error: unknown) =>
        notify({ tone: "error", text: errorMessage(error) }),
      );
    return unsubscribe;
  }, [notify]);
  const control = (command: WindowCommand) => {
    void window.beaver
      .windowControl(command)
      .catch((error: unknown) =>
        notify({ tone: "error", text: errorMessage(error) }),
      );
  };
  return (
    <header className="asset-titlebar">
      <div data-native-drag-region>
        <Icon name="assets" />
        <strong>{title}</strong>
      </div>
      <div className="window-controls">
        <button
          className="window-control"
          aria-label="最小化"
          onClick={() => control("minimize")}
        >
          <Icon name="minimize" />
        </button>
        <button
          className="window-control"
          aria-label={maximized ? "还原窗口" : "最大化"}
          onClick={() => control("toggleMaximize")}
        >
          <Icon name={maximized ? "restore" : "maximize"} />
        </button>
        <button
          className="window-control window-close"
          aria-label="关闭制作窗口"
          title="关闭观察窗口，任务继续执行"
          onClick={() => control("close")}
        >
          <Icon name="close" />
        </button>
      </div>
    </header>
  );
}
