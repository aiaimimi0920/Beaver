import {
  app,
  BrowserWindow,
  Tray,
  Menu,
  nativeImage,
  ipcMain,
  protocol,
  net,
  session,
  safeStorage,
} from "electron";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { z } from "zod";
import { Store } from "../core/store";
import { Preferences } from "../core/settings";
import { Files, ProjectLocks, safePath } from "../core/files";
import { Tasks } from "../core/tasks";
import { Projects } from "../core/projects";
import { Games } from "../core/game";
import { Api } from "./api";
import { windowCommands } from "../shared/window";

if (process.env.BEAVER_DATA_DIR)
  app.setPath("userData", path.resolve(process.env.BEAVER_DATA_DIR));
if (process.platform === "win32")
  app.setAppUserModelId("studio.beaver.desktop");
protocol.registerSchemesAsPrivileged([
  {
    scheme: "beaver-asset",
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
      stream: true,
      corsEnabled: true,
    },
  },
]);
let window: BrowserWindow | undefined;
let tray: Tray | undefined;
let quitting = false;
let tasks: Tasks | undefined;
let games: Games | undefined;
let store: Store | undefined;
let api: Api | undefined;
const requests = new Set<Promise<unknown>>();
if (!app.requestSingleInstanceLock()) app.quit();
else {
  app.on("second-instance", () => {
    window?.show();
    window?.focus();
  });
  app.on("window-all-closed", () => {});
  app.on("activate", () => window?.show());
  app.on("before-quit", (event) => {
    if (!quitting) {
      event.preventDefault();
      void exit();
    }
  });
  void app
    .whenReady()
    .then(async () => {
      const root = app.getPath("userData");
      store = new Store(root);
      const resources = path.join(app.getAppPath(), "resources");
      const branding = path.join(resources, "branding");
      const windowIcon = nativeImage.createFromPath(
        path.join(branding, "beaver-256.png"),
      );
      if (windowIcon.isEmpty()) throw new Error("Beaver brand icon is missing");
      const prefs = new Preferences(store, {
        encrypt: (text) => {
          if (
            !safeStorage.isEncryptionAvailable() ||
            (process.platform === "linux" &&
              safeStorage.getSelectedStorageBackend() === "basic_text")
          )
            throw new Error("系统安全凭据存储不可用，拒绝明文保存 API Key");
          return safeStorage.encryptString(text).toString("base64");
        },
        decrypt: (cipher) =>
          safeStorage.decryptString(Buffer.from(cipher, "base64")),
      });
      const projects = new Projects(store, resources);
      games = new Games(prefs);
      let pending: NodeJS.Timeout | undefined;
      const notify = () => {
        if (!pending)
          pending = setTimeout(() => {
            pending = undefined;
            if (window && !window.isDestroyed())
              window.webContents.send("beaver:changed");
          }, 150);
      };
      tasks = new Tasks(
        store,
        new Files(root),
        new ProjectLocks(),
        prefs,
        resources,
        path.join(__dirname, "mcp.cjs"),
        notify,
      );
      await tasks.ready;
      api = new Api(store, projects, tasks, prefs, games);
      ipcMain.handle("beaver:call", async (event, request: unknown) => {
        try {
          if (quitting) throw new Error("应用正在退出");
          if (
            event.sender !== window?.webContents ||
            event.senderFrame !== window.webContents.mainFrame
          )
            throw new Error("不可信 IPC 来源");
          const r = z
            .object({
              method: z.string().max(80),
              input: z.unknown().optional(),
            })
            .parse(request);
          const operation = api!.call(r.method, r.input);
          requests.add(operation);
          let value: unknown;
          try {
            value = await operation;
          } finally {
            requests.delete(operation);
          }
          if (
            r.method !== "state" &&
            r.method !== "assets" &&
            r.method !== "task.events" &&
            r.method !== "tools.setupStatus"
          )
            notify();
          return { ok: true, value };
        } catch (e) {
          return {
            ok: false,
            error: prefs.redact(e instanceof Error ? e.message : String(e)),
          };
        }
      });
      protocol.handle("beaver-asset", async (request) => {
        try {
          const url = new URL(request.url);
          const root = url.hostname.startsWith("task-")
            ? tasks!.get(url.hostname.slice(5)).workspace
            : projects.get(url.hostname).path;
          const relative = decodeURIComponent(url.pathname.slice(1));
          const file = await safePath(root, relative);
          return net.fetch(pathToFileURL(file).toString());
        } catch {
          return new Response("Not found", { status: 404 });
        }
      });
      session.defaultSession.setPermissionRequestHandler(
        (_wc, _permission, callback) => callback(false),
      );
      window = new BrowserWindow({
        width: 1120,
        height: 760,
        useContentSize: true,
        minWidth: 960,
        minHeight: 680,
        frame: false,
        title: "Beaver",
        icon: windowIcon,
        backgroundColor: "#06080d",
        show: false,
        webPreferences: {
          preload: path.join(__dirname, "preload.cjs"),
          contextIsolation: true,
          nodeIntegration: false,
          sandbox: true,
        },
      });
      // Windows may round the initial frameless client size at fractional DPI.
      window.setContentSize(1120, 760);
      ipcMain.handle("beaver:window", (event, input: unknown) => {
        if (quitting) throw new Error("应用正在退出");
        if (
          !window ||
          event.sender !== window.webContents ||
          event.senderFrame !== window.webContents.mainFrame
        )
          throw new Error("不可信 IPC 来源");
        const command = z.enum(windowCommands).parse(input);
        if (command === "minimize") window.minimize();
        if (command === "toggleMaximize") {
          if (window.isMaximized()) window.unmaximize();
          else window.maximize();
        }
        // Keep the same close-to-tray path as Alt+F4 and the native close event.
        if (command === "close") window.close();
        return { maximized: window.isMaximized() };
      });
      const notifyWindowState = () => {
        if (window && !window.isDestroyed())
          window.webContents.send("beaver:window-state", {
            maximized: window.isMaximized(),
          });
      };
      window.on("maximize", notifyWindowState);
      window.on("unmaximize", notifyWindowState);
      window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
      window.webContents.on("will-navigate", (e) => e.preventDefault());
      window.on("close", (e) => {
        if (!quitting) {
          e.preventDefault();
          window?.hide();
        }
      });
      await window.loadFile(path.join(__dirname, "index.html"));
      if (!process.env.BEAVER_TEST_HIDE) window.show();
      tray = new Tray(
        path.join(
          branding,
          process.platform === "win32" ? "beaver.ico" : "beaver-32.png",
        ),
      );
      tray.setToolTip("Beaver · 关闭窗口后任务继续");
      tray.setContextMenu(
        Menu.buildFromTemplate([
          { label: "打开 Beaver", click: () => window?.show() },
          { type: "separator" },
          {
            label: "退出程序（停止任务，可下次继续）",
            click: () => void exit(),
          },
        ]),
      );
      tray.on("double-click", () => window?.show());
      Menu.setApplicationMenu(null);
    })
    .catch((error) => {
      console.error(error);
      quitting = true;
      app.exit(1);
    });
}
async function exit(): Promise<void> {
  if (quitting) return;
  quitting = true;
  try {
    api?.shutdown();
    await Promise.all([tasks?.shutdown(), games?.shutdown()]);
    await Promise.allSettled([...requests]);
  } finally {
    store?.close();
    tray?.destroy();
    app.quit();
  }
}
