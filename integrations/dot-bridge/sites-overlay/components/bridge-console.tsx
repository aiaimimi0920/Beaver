"use client";
import { useEffect, useState } from "react";
type Task = {
  id: string;
  state: string;
  delivery: string;
  result: string | null;
};
export default function BridgeConsole() {
  const [tasks, setTasks] = useState<Task[]>([]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  async function refresh() {
    try {
      const r = await fetch("/api/demo");
      const d = (await r.json()) as { error?: string; tasks: Task[] };
      if (!r.ok) throw Error(d.error);
      setTasks(d.tasks);
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => {
    void refresh();
  }, []);
  async function probe() {
    setBusy(true);
    setError("");
    try {
      const r = await fetch("/api/demo", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ request_id: "probe_" + crypto.randomUUID() }),
      });
      const d = (await r.json()) as { error?: string; tasks: Task[] };
      if (!r.ok) throw Error(d.error);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <main className="mx-auto max-w-4xl p-6 sm:p-12">
      <p className="text-sm font-semibold tracking-widest text-cyan-600">
        BEAVER / DOT
      </p>
      <h1 className="mt-3 text-3xl font-bold">连接验证台</h1>
      <p className="mt-4 text-lg text-slate-600">
        只测试任务通知与结果回传。不会调用 Codex，也不会生成或修改模型。
      </p>
      <section className="mt-8 rounded-2xl border border-slate-200 bg-white p-6">
        <h2 className="text-xl font-semibold">先连接插件，再订阅测试事件</h2>
        <p className="my-3 text-slate-600">
          只有 dot 实际收到事件，并写回
          BEAVER_DOT_SMOKE_OK，才算一次真实闭环完成。通知已接收不等于任务已完成。
        </p>
        <div className="flex flex-wrap gap-3">
          <button
            disabled={busy}
            onClick={probe}
            className="rounded-lg bg-slate-900 px-5 py-3 text-white disabled:opacity-50"
          >
            {busy ? "正在发起…" : "发起无害测试"}
          </button>
          <button onClick={refresh} className="rounded-lg border px-5 py-3">
            刷新结果
          </button>
        </div>
        {error && (
          <p role="alert" className="mt-4 text-red-700">
            {error}
          </p>
        )}
      </section>
      <section className="mt-8">
        <h2 className="text-xl font-semibold">最近请求</h2>
        {!tasks.length && <p className="mt-3 text-slate-500">尚未发起测试。</p>}
        {tasks.map((t) => (
          <article key={t.id} className="mt-4 rounded-xl border bg-white p-5">
            <p className="break-all font-mono text-sm">{t.id}</p>
            <p className="mt-2 font-semibold">
              {t.state === "completed" ? "已收到结果" : "等待 dot 回传"}
            </p>
            <p className="mt-2 break-all text-sm text-slate-600">
              {t.delivery === "no_active_subscription"
                ? "尚无有效订阅，请先在 dot 中连接插件并订阅。"
                : t.delivery}
            </p>
            {t.result && (
              <p className="mt-2 font-mono text-cyan-700">{t.result}</p>
            )}
          </article>
        ))}
      </section>
      <p className="mt-10 text-sm text-slate-500">
        私有试验接口。当前测试不证明生成质量或用量计费归属。
      </p>
    </main>
  );
}
