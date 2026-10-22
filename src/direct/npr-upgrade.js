"use strict";

function resolveNprUpgradeTarget(state, scope, taskId) {
  const projects = state?.projects ?? [];
  if (scope === "project") {
    if (projects.length !== 1) throw Error("依赖升级需要明确且唯一的工程目标");
    return { id: projects[0].id };
  }
  if (scope !== "task" || !taskId) throw Error("请选择任务工作副本");
  const task = state?.tasks?.find((item) => item.id === taskId);
  if (!task || !projects.some((project) => project.id === task.projectId))
    throw Error("选中任务与当前工程列表不匹配，请重新读取");
  return { id: task.projectId, taskId };
}

// The native host owns quiescence, trusted manifests, journaling and rollback.
function createNprUpgradeActions(call, getTarget) {
  let inspected = null;
  const sameTarget = (a, b) =>
    a.id === b.id && (a.taskId ?? null) === (b.taskId ?? null);
  const eligible = () => {
    if (!inspected || !sameTarget(inspected.target, getTarget())) return false;
    const r = inspected.result;
    return (
      r.eligible === true &&
      /^[a-f0-9]{40}$/.test(r.sourceCommit) &&
      /^[a-f0-9]{64}$/.test(r.runtimeSha256)
    );
  };
  return {
    clear() {
      inspected = null;
    },
    canUpgrade() {
      try {
        return eligible();
      } catch {
        return false;
      }
    },
    async inspect() {
      inspected = null;
      const target = { ...getTarget() };
      if (!target.id) throw Error("先读取当前工程");
      const result = await call("project.npr.inspectUpgrade", target);
      if (
        result.target?.projectId !== target.id ||
        (result.target?.taskId ?? null) !== (target.taskId ?? null)
      )
        throw Error("依赖检查返回了不同目标");
      if (sameTarget(target, getTarget())) inspected = { target, result };
      return result;
    },
    async upgrade() {
      if (!eligible()) throw Error("先检查同一目标，并解决原生升级阻塞");
      const { target, result } = inspected;
      // Any uncertain result requires a fresh inspection, never a blind repeat.
      inspected = null;
      return call("project.npr.upgrade", {
        ...target,
        expectedOldCommit: result.sourceCommit,
        expectedRuntimeSha256: result.runtimeSha256,
      });
    },
  };
}

if (typeof module !== "undefined")
  module.exports = { createNprUpgradeActions, resolveNprUpgradeTarget };
