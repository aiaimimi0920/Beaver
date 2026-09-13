import { DatabaseSync } from "node:sqlite";
import { mkdirSync } from "node:fs";
import path from "node:path";
import type { Task, TaskEvent } from "../shared/types";

export class Store {
  private db: DatabaseSync;
  constructor(readonly root: string) {
    mkdirSync(root, { recursive: true });
    this.db = new DatabaseSync(path.join(root, "beaver.sqlite"));
    this.db.exec(
      "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; CREATE TABLE IF NOT EXISTS entities (kind TEXT NOT NULL, id TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(kind,id)); CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY, task TEXT NOT NULL, time TEXT NOT NULL, kind TEXT NOT NULL, text TEXT NOT NULL); CREATE INDEX IF NOT EXISTS task_events ON events(task,seq);",
    );
  }
  get<T>(kind: string, id: string): T | undefined {
    const row = this.db
      .prepare("SELECT value FROM entities WHERE kind=? AND id=?")
      .get(kind, id);
    return row ? (JSON.parse(String(row.value)) as T) : undefined;
  }
  list<T>(kind: string): T[] {
    return this.db
      .prepare("SELECT value FROM entities WHERE kind=? ORDER BY rowid DESC")
      .all(kind)
      .map((row) => JSON.parse(String(row.value)) as T);
  }
  put(kind: string, id: string, value: unknown): void {
    this.db
      .prepare(
        "INSERT INTO entities(kind,id,value) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
      )
      .run(kind, id, JSON.stringify(value));
  }
  remove(kind: string, id: string): void {
    this.db.prepare("DELETE FROM entities WHERE kind=? AND id=?").run(kind, id);
  }
  transaction(work: () => void): void {
    this.db.exec("BEGIN IMMEDIATE");
    try {
      work();
      this.db.exec("COMMIT");
    } catch (error) {
      this.db.exec("ROLLBACK");
      throw error;
    }
  }
  event(id: string, kind: string, text: string): void {
    this.db
      .prepare("INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)")
      .run(id, new Date().toISOString(), kind, text.slice(0, 32000));
  }
  events(id: string): TaskEvent[] {
    return this.db
      .prepare(
        "SELECT time,kind,text FROM (SELECT seq,time,kind,text FROM events WHERE task=? ORDER BY seq DESC LIMIT 500) ORDER BY seq",
      )
      .all(id)
      .map((r) => ({
        time: String(r.time),
        kind: String(r.kind),
        text: String(r.text),
      }));
  }
  recover(): void {
    for (const task of this.list<Task>("task"))
      if (task.status === "waitingChildren") {
        task.planPaused = true;
        this.put("task", task.id, task);
      }
    for (const task of this.list<Task>("task"))
      if (task.status === "running" || task.status === "queued") {
        task.status = "interrupted";
        task.error = "应用退出或执行进程中断，可继续已有任务。";
        this.put("task", task.id, task);
      }
  }
  close(): void {
    this.db.close();
  }
}
