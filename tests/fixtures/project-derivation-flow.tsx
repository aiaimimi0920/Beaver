import {
  state,
  tick,
  check,
  button,
  click,
  input,
  mount,
  last,
} from "./derivation-fixture";

async function verify() {
  await mount("new");
  check(button("准备独立副本").disabled, "Preparation requires inspection");
  await click("选择源项目");
  check(button("检查派生源项目").disabled, "Cancelled picker stays empty");
  state.directory = "C:/source";
  await click("选择源项目");
  await click("检查派生源项目");
  await input("组装准备目录", "C:/preparation");
  await input("组装目标目录", "C:/assembly");
  await input("离线源项目目录", "C:/changed");
  check(
    button("准备独立副本").disabled,
    "Source changes invalidate inspection",
  );
  await click("检查派生源项目");
  state.hold = true;
  const before = state.requests.length;
  button("准备独立副本").click();
  button("准备独立副本").click();
  await tick();
  check(state.requests.length === before + 1, "Suppress duplicate preparation");
  last("migration.prepareDerivation", {
    source: "C:/source",
    sourceProjectId: "original",
    targetProjectId: "derived",
    requestId: "stable-request",
    preparation: "C:/preparation",
  });
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
  );
  document.querySelector<HTMLButtonElement>('[aria-label="关闭弹窗"]')!.click();
  check(
    state.closed === 0 && button("关闭").disabled,
    "In-flight derivation cannot dismiss",
  );
  state.hold = false;
  state.release?.();
  await tick();
  await click("组装到新目录");
  last("migration.assembleDerivation", {
    preparation: "C:/preparation",
    destination: "C:/assembly",
  });
  check(
    document.body.textContent?.includes("C:/assembly/project"),
    "Display final bound path",
  );
  const assembled = state.requests.length;
  await click("启用组装副本");
  await click("取消组装启用");
  check(
    state.requests.length === assembled,
    "Only explicit confirmation activates",
  );
  await click("启用组装副本");
  await input("组装目标目录", "C:/other");
  check(
    !document.body.textContent?.includes("确认启用新身份副本"),
    "Path change clears authorization and receipt",
  );
  await input("组装目标目录", "C:/assembly");
  await click("核验已有组装副本");
  await click("启用组装副本");
  await click("确认启用组装副本");
  last("migration.activateAssembly", {
    preparation: "C:/preparation",
    destination: "C:/assembly",
  });
  check(
    state.requests.every((r) => r.method !== "migration.registerAssembly"),
    "Activation never registers implicitly",
  );
  await click("登记并恢复运行时");
  check(
    document.body.textContent?.includes("组装副本已登记。运行时恢复失败"),
    "Separate durable registration from recovery failure",
  );
  state.runtimeReady = true;
  const retry = state.requests.length;
  await click("登记并恢复运行时");
  check(
    state.requests.length === retry + 1,
    "Registration retry must not inspect old inventory",
  );
  last("migration.registerAssembly", {
    preparation: "C:/preparation",
    destination: "C:/assembly",
  });
  await click("打开派生项目");
  check(state.opened === "derived", "Open exact derived identity");

  await mount("lost-preparation");
  await input("离线源项目目录", "C:/source");
  await click("检查派生源项目");
  await input("组装准备目录", "C:/lost");
  state.failure = "migration.prepareDerivation";
  await click("准备独立副本");
  check(
    button("准备独立副本").disabled,
    "Unknown result cannot overwrite same preparation",
  );
  check(
    document.body.textContent?.includes("保留 C:/lost"),
    "Retain failed preparation path",
  );
  state.failure = "";
  await click("核验已有准备副本");
  check(
    document.body.textContent?.includes("原请求 stable-request"),
    "Recover original identity without source",
  );
  await input("组装目标目录", "C:/lost-assembly");
  state.failure = "migration.assembleDerivation";
  await click("组装到新目录");
  check(
    button("组装到新目录").disabled,
    "Unknown assembly cannot overwrite same directory",
  );
  state.failure = "";
  state.activated = true;
  await click("核验已有组装副本");
  check(
    document.body.textContent?.includes("组装已核验：已启用"),
    "Recover lost activation response without activating again",
  );
  state.failure = "migration.registerAssembly";
  await click("登记并恢复运行时");
  check(
    ![...document.querySelectorAll("button")].some(
      (n) => n.textContent === "核验已有组装副本",
    ),
    "Lost registration response switches to exact replay, not inventory validation",
  );
  state.failure = "";
  await mount("resume-after-reopen");
  await input("派生操作", "registered");
  await input("组装准备目录", "C:/lost");
  await input("组装目标目录", "C:/lost-assembly");
  const reopened = state.requests.length;
  await click("登记并恢复运行时");
  check(
    state.requests.length === reopened + 1,
    "Reopened dialog resumes registration without old inventory",
  );
  last("migration.registerAssembly", {
    preparation: "C:/lost",
    destination: "C:/lost-assembly",
  });
  await click("打开派生项目");
  check(state.opened === "derived", "Reopened registration can open project");
  return "PASS: offline source inspection, request identity, duplicate/close guards, complete derivation, explicit activation, path invalidation, durable failure recovery, lost-response replay, reopen, open project";
}
Object.assign(window, { migrationResult: verify() });
