import { env } from "cloudflare:workers";
export type Subscription = {
  id: string;
  owner: string;
  callback: string;
  secret: string;
  expires: number;
};
export type Task = {
  id: string;
  event_id: string;
  created_at: string;
  state: string;
  delivery: string;
  result: string | null;
  completed_at: string | null;
};
function db() {
  return (env as unknown as { DB: D1Database }).DB;
}
export async function subscriptions() {
  return (
    await db()
      .prepare("SELECT * FROM subscriptions WHERE expires > ?")
      .bind(Date.now())
      .all<Subscription>()
  ).results;
}
export async function saveSubscription(s: Subscription) {
  await db()
    .prepare(
      "INSERT INTO subscriptions(id,owner,callback,secret,expires) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET secret=excluded.secret,expires=excluded.expires",
    )
    .bind(s.id, s.owner, s.callback, s.secret, s.expires)
    .run();
}
export async function removeSubscription(owner: string, callback: string) {
  await db()
    .prepare("DELETE FROM subscriptions WHERE owner=? AND callback=?")
    .bind(owner, callback)
    .run();
}
export async function listTasks() {
  return (
    await db()
      .prepare("SELECT * FROM tasks ORDER BY created_at DESC LIMIT 20")
      .all<Task>()
  ).results;
}
export async function getTask(id: string) {
  const r = await db()
    .prepare("SELECT * FROM tasks WHERE id=?")
    .bind(id)
    .first<Task>();
  if (!r) throw Error("Task not found");
  return r;
}
export async function createTask(id: string) {
  if (!/^[a-zA-Z0-9_-]{1,80}$/.test(id)) throw Error("Invalid request ID");
  await db()
    .prepare(
      "INSERT INTO tasks(id,event_id,created_at) VALUES(?,?,?) ON CONFLICT(id) DO NOTHING",
    )
    .bind(id, "evt_" + crypto.randomUUID(), new Date().toISOString())
    .run();
  return getTask(id);
}
export async function recordDelivery(id: string, result: string) {
  await db()
    .prepare("UPDATE tasks SET delivery=? WHERE id=?")
    .bind(result, id)
    .run();
}
export async function completeTask(id: string, result: string) {
  if (result !== "BEAVER_DOT_SMOKE_OK")
    throw Error("Only the smoke-test marker is accepted");
  await getTask(id);
  await db()
    .prepare(
      "UPDATE tasks SET state='completed',result=?,completed_at=? WHERE id=? AND state='pending'",
    )
    .bind(result, new Date().toISOString(), id)
    .run();
  return getTask(id);
}
