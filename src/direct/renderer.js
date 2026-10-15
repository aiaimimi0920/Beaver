const $ = (id) => document.getElementById(id);
let state,
  context,
  lastRequest,
  lastJob,
  busy = false;
const print = (value) => {
  const texts = [];
  function collect(v, key = "") {
    if (
      typeof v === "string" &&
      ["stdout", "stderr", "content", "tail", "logTail"].includes(key)
    ) {
      if (key === "tail") {
        try {
          const report = JSON.parse(
            v.slice(v.indexOf("{"), v.lastIndexOf("}") + 1),
          );
          if (Array.isArray(report.objects)) {
            const head = report.objects.filter((o) =>
              o.materials.some((m) => ["Face", "Hair"].includes(m)),
            );
            texts.push(
              "HEAD SOURCE SUMMARY\n" +
                report.definition +
                "\n" +
                JSON.stringify(report.nodes) +
                "\n" +
                head.map((o) => JSON.stringify(o)).join("\n"),
            );
          }
        } catch (_) {
          /* Keep unparsed logs visible below. */
        }
      }
      texts.push(key + ":\n" + v);
    } else if (v && typeof v === "object") {
      for (const [k, child] of Object.entries(v)) collect(child, k);
    }
  }
  collect(value);
  $("result").textContent =
    texts.join("\n\n") +
    "\n\nRAW RECEIPT\n" +
    JSON.stringify(
      {
        taskId: value?.taskId,
        id: value?.id,
        runId: value?.runId,
        revision: value?.revision,
        status: value?.status,
        result: value?.result,
        ...value,
      },
      null,
      2,
    );
  $("result").scrollTop = 0;
};
async function action(fn) {
  if (busy) return;
  busy = true;
  document.querySelectorAll("button").forEach((b) => (b.disabled = true));
  $("status").textContent = "正在等待真实 Core 回执";
  try {
    const r = await fn();
    print(r);
    $("status").textContent = "操作已返回，请检查实际结果";
    return r;
  } catch (e) {
    $("status").textContent = "操作失败或结果未确认";
    print({ error: e.message });
    throw e;
  } finally {
    busy = false;
    document.querySelectorAll("button").forEach((b) => (b.disabled = false));
    updateRole();
  }
}
function updateRole() {
  const r = $("role").value;
  $("create").disabled = busy || r !== "user";
  $("plan").disabled = busy || r !== "planner";
  $("finish").disabled = busy || r !== "executor";
}
async function refresh() {
  state = await window.beaver.call("state");
  const old = $("task").value;
  $("task").replaceChildren();
  for (const t of state.tasks) {
    const o = document.createElement("option");
    o.value = t.id;
    o.textContent = t.title + " · " + t.status;
    $("task").append(o);
  }
  if (state.tasks.some((t) => t.id === old)) $("task").value = old;
  return state;
}
async function getContext() {
  context = await window.beaver.call("external.context", {
    taskId: $("task").value,
  });
  $("details").textContent = JSON.stringify(context, null, 2);
  return context;
}
async function submit(method, args) {
  if (!context || context.taskId !== $("task").value)
    throw Error("先读取所选任务的当前上下文");
  lastRequest = {
    taskId: context.taskId,
    runId: context.runId,
    revision: context.revision,
    requestId: crypto.randomUUID(),
    arguments: args,
  };
  localStorage.setItem("lastRequest", JSON.stringify(lastRequest));
  const r = await window.beaver.call(method, lastRequest);
  if (Number.isInteger(r.revision)) context.revision = r.revision;
  if (r.result?.jobId) lastJob = r.result.jobId;
  if (r.status === "failed")
    $("status").textContent = "Core 操作失败，保留回执";
  return r;
}
$("role").onchange = updateRole;
updateRole();
$("setup").onclick = () =>
  action(() => window.beaver.call("recovery.setup")).catch(() => {});
$("refresh").onclick = () => action(refresh).catch(() => {});
$("context").onclick = () => action(getContext).catch(() => {});
$("history").onclick = () =>
  action(() =>
    window.beaver.call("external.history", { taskId: $("task").value }),
  ).catch(() => {});
$("create").onclick = () =>
  action(async () => {
    if (!state?.projects?.length) throw Error("先恢复/读取工程");
    return window.beaver.call("task.create", {
      projectId: state.projects[0].id,
      prompt: $("goal").value,
      title: "Restore original NPR head",
      direction: "visual",
      capability: "code",
      executionMode: "external-agent",
      decompose: true,
      autoAccept: true,
      maxMinutes: 240,
      askRatio: 0,
    });
  }).catch(() => {});
$("continue").onclick = () =>
  action(() =>
    window.beaver.call("task.continue", {
      id: $("task").value,
      text: $("goal").value,
    }),
  ).catch(() => {});
$("execute").onclick = () =>
  action(() =>
    submit("external.tool", {
      tool: $("tool").value,
      arguments: JSON.parse($("args").value),
    }),
  ).catch(() => {});
$("plan").onclick = () =>
  action(() =>
    submit("external.submitPlan", JSON.parse($("args").value)),
  ).catch(() => {});
$("finish").onclick = () =>
  action(() => submit("external.finish", { outcome: "completed" })).catch(
    () => {},
  );
$("receipt").onclick = () =>
  action(() => {
    const r = lastRequest || JSON.parse(localStorage.getItem("lastRequest"));
    if (!r) throw Error("No request");
    return window.beaver.call("external.receipt", {
      taskId: r.taskId,
      runId: r.runId,
      requestId: r.requestId,
    });
  }).catch(() => {});
$("poll").onclick = () => {
  if (!lastJob) {
    print({ error: "No job ID in last result; inspect receipt" });
    return;
  }
  $("tool").value = "blender.poll";
  $("args").value = JSON.stringify({ jobId: lastJob });
};
$("file").onchange = async () => {
  const f = $("file").files[0];
  if (f) {
    const s = await f.text();
    const value = JSON.parse(s);
    if (typeof value.userGoal === "string") $("goal").value = value.userGoal;
    else if (typeof value.previewDefinition === "string")
      $("preview-definition").value = value.previewDefinition;
    else $("args").value = s;
    $("file").value = "";
  }
};

$("open-preview").onclick = () =>
  action(async () => {
    if (!context) throw Error("先读取当前任务上下文");
    const definition = $("preview-definition").value;
    const checked = await submit("external.tool", {
      tool: "workflow.run",
      arguments: { workflow: "npr-character", action: "validate", definition },
    });
    if (checked.status !== "succeeded" || checked.result?.ok !== true)
      return checked;
    return window.beaver.call("preview.open", {
      taskId: context.taskId,
      runId: context.runId,
      definition,
    });
  }).catch(() => {});
$("preview-status").onclick = () =>
  action(() => window.beaver.call("preview.status")).catch(() => {});
$("close-preview").onclick = () =>
  action(() => window.beaver.call("preview.close")).catch(() => {});
