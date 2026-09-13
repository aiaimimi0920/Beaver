import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import readline from "node:readline";
import { EventEmitter } from "node:events";
import { terminate } from "./process";

export function object(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
export class CodexRpc extends EventEmitter {
  private proc?: ChildProcessWithoutNullStreams;
  private sequence = 0;
  private closed = false;
  private closing?: Promise<void>;
  private pending = new Map<
    number,
    {
      resolve: (v: unknown) => void;
      reject: (e: Error) => void;
      timer: NodeJS.Timeout;
    }
  >();
  async connect(
    command: string,
    env: NodeJS.ProcessEnv,
    cwd: string,
  ): Promise<void> {
    if (this.closed) throw new Error("连接已关闭");
    this.proc = spawn(command, ["app-server", "--listen", "stdio://"], {
      env,
      cwd,
      stdio: "pipe",
      windowsHide: true,
      detached: process.platform !== "win32",
    });
    this.proc.on("error", (e) => this.fail(e));
    this.proc.on("close", () => {
      this.fail(new Error("Codex 执行进程已退出"));
      this.emit("exit");
    });
    this.proc.stderr.on("data", (b: Buffer) =>
      this.emit("log", b.toString().slice(-8000)),
    );
    readline.createInterface({ input: this.proc.stdout }).on("line", (line) => {
      try {
        const msg = object(JSON.parse(line));
        if (typeof msg.method === "string") {
          if (typeof msg.id === "string" || typeof msg.id === "number") {
            if (
              !this.emit(
                "serverRequest",
                msg.id,
                msg.method,
                object(msg.params),
              )
            )
              this.rejectRequest(msg.id, "Unsupported interactive request");
          } else this.emit("notification", msg.method, object(msg.params));
        } else if (typeof msg.id === "number") {
          const request = this.pending.get(msg.id);
          if (!request) return;
          clearTimeout(request.timer);
          this.pending.delete(msg.id);
          if (msg.error)
            request.reject(
              new Error(String(object(msg.error).message ?? "Codex RPC error")),
            );
          else request.resolve(msg.result);
        }
      } catch {
        this.emit("log", "忽略无法解析的 Codex 输出");
      }
    });
    await this.request("initialize", {
      clientInfo: { name: "beaver_studio", title: "Beaver", version: "0.1.0" },
      capabilities: { experimentalApi: true },
    });
    this.send({ method: "initialized", params: {} });
  }
  request(method: string, params: unknown): Promise<unknown> {
    const id = ++this.sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Codex ${method} 请求超时`));
      }, 45000);
      this.pending.set(id, { resolve, reject, timer });
      try {
        this.send({ id, method, params });
      } catch (e) {
        clearTimeout(timer);
        this.pending.delete(id);
        reject(e);
      }
    });
  }
  rejectRequest(id: string | number, message: string): void {
    this.send({ id, error: { code: -32000, message } });
  }
  respond(id: string | number, result: unknown): void {
    this.send({ id, result });
  }
  private send(value: unknown): void {
    if (!this.proc?.stdin.writable) throw new Error("Codex 未连接");
    this.proc.stdin.write(JSON.stringify(value) + "\n");
  }
  private fail(error: Error): void {
    for (const r of this.pending.values()) {
      clearTimeout(r.timer);
      r.reject(error);
    }
    this.pending.clear();
  }
  async close(): Promise<void> {
    this.closed = true;
    this.closing ??= (async () => {
      if (this.proc) await terminate(this.proc);
      this.fail(new Error("连接已关闭"));
    })();
    await this.closing;
  }
}
