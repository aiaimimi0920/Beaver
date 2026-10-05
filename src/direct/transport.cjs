const { spawn } = require("node:child_process");
const readline = require("node:readline");
const fs = require("node:fs/promises");
class CoreTransport {
  constructor(binary, dataDir) {
    this.seq = 0;
    this.pending = null;
    this.closed = false;
    this.audit = dataDir + "/client-receipts.jsonl";
    this.child = spawn(binary, ["--data-dir", dataDir], {
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.errors = "";
    this.child.stderr.on("data", (b) => {
      this.errors = (this.errors + b).slice(-16000);
    });
    readline.createInterface({ input: this.child.stdout }).on("line", (s) => {
      let r;
      try {
        r = JSON.parse(s);
      } catch {
        this.fail("Invalid Core JSON response");
        return;
      }
      const p = this.pending;
      if (!p || r.id !== p.id) {
        this.fail("Unexpected Core response identity");
        return;
      }
      this.pending = null;
      fs.appendFile(
        this.audit,
        JSON.stringify({ at: new Date().toISOString(), response: r }) + "\n",
      ).catch(() => {});
      if (r.error) p.reject(new Error(r.error.message));
      else p.resolve(r.result);
    });
    this.child.on("error", (e) => this.fail(e.message));
    this.child.on("exit", (code) => {
      this.closed = true;
      this.fail("Core exited " + code + " " + this.errors);
    });
  }
  fail(message) {
    if (this.pending) {
      this.pending.reject(new Error(message));
      this.pending = null;
    }
  }
  call(method, input = {}) {
    if (this.closed) return Promise.reject(new Error("Core closed"));
    if (this.pending)
      return Promise.reject(
        new Error("Core call already pending; wait for its receipt"),
      );
    const id = ++this.seq;
    const line = JSON.stringify({ id, method, input }) + "\n";
    if (Buffer.byteLength(line) > 1048576)
      return Promise.reject(new Error("Request exceeds Core frame limit"));
    return new Promise((resolve, reject) => {
      this.pending = { id, resolve, reject };
      this.child.stdin.write(line, (e) => {
        if (e) this.fail(e.message);
      });
    });
  }
}
module.exports = { CoreTransport };
