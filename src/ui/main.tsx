import "./native-bridge";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { NotificationProvider } from "./Notifications";
import { AssetTaskWindow } from "./asset-task/AssetTaskWindow";
import { ObjectFrameworkPreview } from "./object-preview/ObjectFrameworkPreview";
import "./style.css";
const root = document.getElementById("root");
const assetTask = new URLSearchParams(window.location.search).get("assetTask");
if (root)
  createRoot(root).render(
    !window.beaver &&
      new URLSearchParams(window.location.search).get("preview") ===
        "objects" ? (
      <ObjectFrameworkPreview />
    ) : window.beaver ? (
      <NotificationProvider>
        {assetTask && "__TAURI__" in window ? (
          <AssetTaskWindow key={assetTask} id={assetTask} />
        ) : (
          <App />
        )}
      </NotificationProvider>
    ) : (
      <main className="startup">
        <h1>Beaver</h1>
        <p>请通过 Beaver 桌面程序打开。本页面没有浏览器模拟后端。</p>
        <a href="?preview=objects">打开创作、对象与制造 UI 预览（测试数据）</a>
      </main>
    ),
  );
