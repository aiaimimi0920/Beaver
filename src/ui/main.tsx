import "./native-bridge";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { NotificationProvider } from "./Notifications";
import "./style.css";
const root = document.getElementById("root");
if (root)
  createRoot(root).render(
    window.beaver ? (
      <NotificationProvider>
        <App />
      </NotificationProvider>
    ) : (
      <main className="startup">
        <h1>Beaver</h1>
        <p>请通过 Beaver 桌面程序打开。本页面没有浏览器模拟后端。</p>
      </main>
    ),
  );
