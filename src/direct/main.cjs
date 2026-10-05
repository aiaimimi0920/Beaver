const { app, BrowserWindow, ipcMain } = require("electron");
const path = require("node:path");
const fs = require("node:fs/promises");
const { execFile } = require("node:child_process");
const { promisify } = require("node:util");
const { CoreTransport } = require("./transport.cjs");
const run = promisify(execFile);
const { HeadPreview } = require("./preview.cjs");
const base = path.resolve(__dirname, "../../..");
const binary = path.join(base, "restored-tools/beaver/beaver-headless");
const engine = path.join(
  base,
  "restored-tools/midot/midot-linux-x86_64/godot.linuxbsd.editor.x86_64",
);
const data = path.join(base, "beaver-restored-host");
const projects = path.join(base, "restored-projects");
app.setPath("userData", path.join(base, "beaver-restored-gui"));
const preview = new HeadPreview(engine, path.join(base, "recovery-status"));
let core,
  win,
  closing = false;
app.whenReady().then(async () => {
  await fs.mkdir(data, { recursive: true });
  await fs.mkdir(projects, { recursive: true });
  core = new CoreTransport(binary, data);
  win = new BrowserWindow({
    width: 1280,
    height: 960,
    title: "Beaver · dot 恢复工作台",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  ipcMain.handle("beaver-direct", async (event, { method, input }) => {
    if (
      event.sender !== win.webContents ||
      event.senderFrame !== win.webContents.mainFrame
    )
      throw Error("Untrusted frame");
    if (method === "preview.open") {
      const context = await core.call("external.context", {
        taskId: input.taskId,
      });
      if (context.runId !== input.runId || context.status !== "active")
        throw Error("Preview requires the active task context");
      return preview.start(context, projects, input.definition);
    }
    if (method === "preview.status") return preview.status();
    if (method === "preview.close") {
      await preview.close();
      return { status: "closed" };
    }
    if (method === "recovery.setup") {
      const settings = await core.call("settings.get");
      settings.tools.godot = engine;
      settings.tools.blender = "/usr/bin/blender";
      settings.tools.node = process.execPath;
      settings.maxParallel = 1;
      await core.call("settings.save", { settings, keys: {} });
      const state = await core.call("state");
      if (state.tasks.length || state.projects.length > 1)
        throw Error("Recovery target has task history; do not overwrite it");
      const project =
        state.projects[0] ||
        (await core.call("project.create", {
          parent: projects,
          name: "Aster Restored",
          template: "blank",
          npr: { godot: engine },
        }));
      if (project.path !== path.join(projects, "Aster Restored"))
        throw Error("Unexpected recovery destination");
      const result = await run(
        "python3",
        [
          path.join(__dirname, "restore.py"),
          path.join(base, "recovered-library/Aster-NPR-lab-source.zip"),
          project.path,
        ],
        { maxBuffer: 1024 * 1024 },
      );
      return { project, restore: JSON.parse(result.stdout) };
    }
    return core.call(method, input || {});
  });
  await win.loadFile(path.join(__dirname, "index.html"));
  win.on("close", (event) => {
    if (!closing) {
      event.preventDefault();
      closing = true;
      preview
        .close()
        .then(() => core.call("shutdown"))
        .then(() => app.quit())
        .catch((e) => {
          closing = false;
          win.webContents.send("close-error", e.message);
        });
    }
  });
});
