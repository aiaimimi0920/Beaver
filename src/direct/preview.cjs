const fs = require("node:fs/promises");
const { createWriteStream } = require("node:fs");
const { spawn } = require("node:child_process");
const path = require("node:path");
const crypto = require("node:crypto");

class HeadPreview {
  constructor(engine, logDirectory) {
    this.engine = engine;
    this.logDirectory = logDirectory;
    this.child = null;
    this.receipt = null;
  }
  async start(context, allowedProjectsRoot, definition) {
    if (this.child)
      throw Error("An owned preview is already running; close it first");
    const workspace = await fs.realpath(context.workspace);
    const root = await fs.realpath(allowedProjectsRoot);
    if (!workspace.startsWith(root + path.sep))
      throw Error("Preview workspace outside project root");
    if (
      typeof definition !== "string" ||
      !/^assets\/aster\/head_recovery_[0-9]+\/aster_definition\.tres$/.test(
        definition,
      )
    )
      throw Error("Unsupported preview definition");
    const definitionPath = await fs.realpath(path.join(workspace, definition));
    if (!definitionPath.startsWith(workspace + path.sep))
      throw Error("Definition outside task workspace");
    const scene = "showcase/aster_head_review.tscn";
    const scenePath = await fs.realpath(path.join(workspace, scene));
    if (!scenePath.startsWith(workspace + path.sep))
      throw Error("Preview scene outside workspace");
    const digest = crypto
      .createHash("sha256")
      .update(await fs.readFile(this.engine))
      .digest("hex");
    if (
      digest !==
      "888d345cda807f7dc6fae1a9aad6fbd256736e086a34d37819a39c19d311c5dd"
    )
      throw Error("MiDot executable identity changed");
    await fs.mkdir(this.logDirectory, { recursive: true });
    const logPath = path.join(
      this.logDirectory,
      "head-preview-" + Date.now() + ".log",
    );
    const output = createWriteStream(logPath, { flags: "wx" });
    const args = [
      "--path",
      workspace,
      "--rendering-method",
      "forward_plus",
      "--audio-driver",
      "Dummy",
      "--resolution",
      "1280x900",
      "res://" + scene,
      "--",
      "res://" + definition,
    ];
    const child = spawn(this.engine, args, {
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.child = child;
    child.stdout.pipe(output, { end: false });
    child.stderr.pipe(output, { end: false });
    this.receipt = {
      pid: child.pid,
      taskId: context.taskId,
      runId: context.runId,
      workspace,
      scene,
      definition,
      engineSha256: digest,
      logPath,
      status: "starting",
    };
    child.once("close", (code, signal) => {
      output.end();
      this.child = null;
      this.receipt = { ...this.receipt, status: "exited", code, signal };
    });
    try {
      await new Promise((resolve, reject) => {
        child.once("spawn", resolve);
        child.once("error", reject);
      });
    } catch (error) {
      output.end();
      this.child = null;
      throw error;
    }
    this.receipt.status = "running";
    return this.receipt;
  }
  async status() {
    if (!this.receipt) return { status: "not_started" };
    const log = await fs.readFile(this.receipt.logPath, "utf8").catch(() => "");
    return { ...this.receipt, logTail: log.slice(-12000) };
  }
  async close() {
    if (!this.child) return;
    const child = this.child;
    const stopped = new Promise((resolve) => child.once("close", resolve));
    child.kill("SIGTERM");
    await stopped;
  }
}
module.exports = { HeadPreview };
