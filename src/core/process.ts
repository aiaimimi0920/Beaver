import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { resolveToolHints, windowsToolHintsScript } from "./tool-discovery";

let pendingWindowsHints: Promise<string[]> | undefined;
function windowsHints(): Promise<string[]> {
  if (!pendingWindowsHints) {
    const powershell = path.join(
      process.env.SystemRoot ?? "C:/Windows",
      "System32/WindowsPowerShell/v1.0/powershell.exe",
    );
    pendingWindowsHints = runCommand(
      powershell,
      [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-EncodedCommand",
        Buffer.from(windowsToolHintsScript, "utf16le").toString("base64"),
      ],
      undefined,
      12000,
    )
      .then(({ code, output }) => {
        if (code !== 0) return [];
        const value: unknown = JSON.parse(output.replace(/^\uFEFF/, "").trim());
        return Array.isArray(value)
          ? value
              .filter((v): v is string => typeof v === "string")
              .slice(0, 500)
          : [];
      })
      .catch(() => [])
      .finally(() => {
        pendingWindowsHints = undefined;
      });
  }
  return pendingWindowsHints;
}

export async function exists(file: string): Promise<boolean> {
  try {
    return (await fs.stat(file)).isFile();
  } catch {
    return false;
  }
}
export async function findTool(
  name: "codex" | "godot" | "blender" | "node",
  configured = "",
): Promise<string> {
  if (configured) {
    if (!(await exists(configured)))
      throw new Error(`工具路径不存在：${configured}`);
    if (
      process.platform === "win32" &&
      name === "godot" &&
      /(?:^|[_.-])(?:template|headless|server)(?:[_.-]|$)/i.test(
        path.basename(configured),
      )
    )
      throw new Error("请选择 Godot 编辑器程序，不能使用导出模板或服务器程序");
    if (
      process.platform === "win32" &&
      (name === "godot" || name === "blender") &&
      /\.(cmd|bat)$/i.test(configured)
    ) {
      const resolved = await resolveToolHints(name, [configured]);
      if (!resolved)
        throw new Error("无法解析工具启动脚本，请选择实际的 EXE 文件");
      return resolved;
    }
    return configured;
  }
  const candidates: string[] = [];
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!dir) continue;
    candidates.push(
      path.join(dir, name + (process.platform === "win32" ? ".exe" : "")),
    );
    if (
      process.platform === "win32" &&
      (name === "godot" || name === "blender")
    )
      candidates.push(
        path.join(dir, name + ".cmd"),
        path.join(dir, name + ".bat"),
        dir,
      );
    if (name === "codex")
      candidates.push(
        path.join(
          dir,
          "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        ),
      );
  }
  if (name === "godot")
    candidates.push(
      process.env.GODOT_PATH ?? "",
      path.join(os.homedir(), "scoop/apps/godot/current/godot.exe"),
      "/Applications/Godot.app/Contents/MacOS/Godot",
    );
  if (name === "godot" && process.platform === "win32") {
    for (const dir of [
      path.join(os.homedir(), "scoop/apps/godot/current"),
      path.join(
        process.env.LOCALAPPDATA ?? os.homedir(),
        "Microsoft/WinGet/Links",
      ),
    ]) {
      try {
        for (const file of await fs.readdir(dir))
          if (/^godot.*\.exe$/i.test(file))
            candidates.push(path.join(dir, file));
      } catch {}
    }
  }
  if (name === "blender") {
    candidates.push(process.env.BLENDER_PATH ?? "");
    candidates.push("/Applications/Blender.app/Contents/MacOS/Blender");
    try {
      const root = path.join(
        process.env.ProgramFiles ?? "C:/Program Files",
        "Blender Foundation",
      );
      for (const dir of await fs.readdir(root))
        candidates.push(path.join(root, dir, "blender.exe"));
    } catch {}
  }
  if (name === "node" && process.platform === "win32")
    candidates.push(
      path.join(
        process.env.ProgramFiles ?? "C:/Program Files",
        "nodejs/node.exe",
      ),
    );
  if (
    process.platform === "win32" &&
    (name === "godot" || name === "blender")
  ) {
    const found = await resolveToolHints(name, candidates);
    if (found) return found;
    const registered = await resolveToolHints(name, await windowsHints());
    if (registered) return registered;
    throw new Error(`未检测到 ${name}，请安装或选择可执行文件`);
  }
  for (const c of candidates) if (await exists(c)) return c;
  throw new Error(`未检测到 ${name}，请安装或选择可执行文件`);
}
export async function terminate(
  proc: ChildProcessWithoutNullStreams,
): Promise<void> {
  if (proc.exitCode !== null || proc.signalCode !== null || !proc.pid) return;
  const closed = new Promise<void>((resolve) =>
    proc.once("close", () => resolve()),
  );
  if (process.platform === "win32")
    await new Promise<void>((resolve) => {
      const killer = spawn(
        "taskkill.exe",
        ["/PID", String(proc.pid), "/T", "/F"],
        { windowsHide: true },
      );
      const timer = setTimeout(() => {
        killer.kill();
        proc.kill();
        resolve();
      }, 5000);
      killer.on("error", () => {
        clearTimeout(timer);
        proc.kill();
        resolve();
      });
      killer.on("close", () => {
        clearTimeout(timer);
        resolve();
      });
    });
  else {
    try {
      process.kill(-proc.pid, "SIGTERM");
    } catch {
      proc.kill();
    }
  }
  let timer: NodeJS.Timeout | undefined;
  const stopped = await Promise.race([
    closed.then(() => true),
    new Promise<false>((resolve) => {
      timer = setTimeout(() => resolve(false), 3000);
    }),
  ]);
  clearTimeout(timer);
  if (!stopped && proc.exitCode === null && proc.signalCode === null) {
    try {
      if (process.platform !== "win32") process.kill(-proc.pid, "SIGKILL");
      else proc.kill("SIGKILL");
    } catch {}
    await Promise.race([
      closed,
      new Promise<void>((resolve) => {
        timer = setTimeout(resolve, 2000);
      }),
    ]);
    clearTimeout(timer);
    if (proc.exitCode === null && proc.signalCode === null)
      throw new Error("无法停止本次工具进程，请检查进程权限");
  }
}
export async function runCommand(
  command: string,
  args: string[],
  cwd?: string,
  timeout = 15000,
  signal?: AbortSignal,
): Promise<{ code: number; output: string }> {
  if (signal?.aborted) throw new Error("工具操作已取消");
  return new Promise((resolve, reject) => {
    const proc = spawn(command, args, {
      cwd,
      windowsHide: true,
      detached: process.platform !== "win32",
      stdio: "pipe",
    });
    let output = "";
    let stopping: Error | undefined;
    const stop = (error: Error) => {
      stopping = error;
      void terminate(proc).catch(reject);
    };
    const abort = () => stop(new Error("工具操作已取消"));
    signal?.addEventListener("abort", abort, { once: true });
    const collect = (b: Buffer) => {
      output = (output + b.toString()).slice(-64000);
    };
    proc.stdout.on("data", collect);
    proc.stderr.on("data", collect);
    const timer = setTimeout(() => {
      stop(new Error("工具操作超时"));
    }, timeout);
    proc.on("error", (e) => {
      clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
      reject(e);
    });
    proc.on("close", (code) => {
      clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
      if (stopping) {
        reject(stopping);
        return;
      }
      resolve({ code: code ?? -1, output });
    });
  });
}
