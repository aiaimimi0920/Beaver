import { demoTasks, type DemoTask } from "./mock-tasks";
import type { PreviewAnnotation } from "./preview-target";

// Keep the chosen task order for this preview session.
type Placement = "before" | "after";
const insertedTasks = new Set<string>();
const objectOrders = new Map<string, string[]>();
const originalTasks = new Map<string, DemoTask>();

export interface IterationContent {
  title: string;
  prompt: string;
  annotations: PreviewAnnotation[];
}

function iterationDetail({ prompt, annotations }: IterationContent) {
  return (
    prompt.trim() ||
    annotations
      .filter((annotation) => annotation.prompt.trim())
      .map(
        (annotation) =>
          `序号 ${annotation.number}：${annotation.prompt.trim()}`,
      )
      .join("\n")
  );
}

function shortTitle(title: string) {
  const characters = Array.from(title);
  return characters.slice(0, 16).join("") + (characters.length > 16 ? "…" : "");
}

// UI-only title suggestion; the model service is not connected yet.
export function suggestMockIterationTitle(content: IterationContent) {
  const detail = iterationDetail(content).replace(/\s+/g, " ");
  return shortTitle(detail.split(/[。！？\n]/u)[0] || "新的修改建议");
}

function taskContent(content: IterationContent) {
  const detail = iterationDetail(content);
  if (!detail) return null;
  const title = content.title.trim() || suggestMockIterationTitle(content);
  return {
    title,
    summary: shortTitle(title),
    prompt: content.prompt,
    detail,
    annotations: content.annotations.map((annotation) => ({ ...annotation })),
  };
}

export function iterationOrder(task: DemoTask) {
  if (task.status === "已验收") return 0;
  if (task.status === "排队中" || task.status === "等待依赖") return 2;
  return 1;
}

export function objectIterations(objectId: string) {
  const tasks = demoTasks.filter((task) => task.objectId === objectId);
  const iterations = tasks
    .filter((task) => !tasks.some((child) => child.parentId === task.id))
    .sort((a, b) => iterationOrder(a) - iterationOrder(b));
  const order = objectOrders.get(objectId);
  if (!order) return iterations;
  const remaining = new Map(iterations.map((task) => [task.id, task]));
  const ordered = order.flatMap((id) => {
    const task = remaining.get(id);
    remaining.delete(id);
    return task ? [task] : [];
  });
  return [...ordered, ...remaining.values()];
}

export function moveMockIteration(
  objectId: string,
  taskId: string,
  beforeId: string | null,
) {
  const tasks = objectIterations(objectId);
  if (taskId === beforeId || !tasks.some((task) => task.id === taskId)) return;
  const order = tasks.map((task) => task.id).filter((id) => id !== taskId);
  const index = beforeId === null ? order.length : order.indexOf(beforeId);
  if (index < 0) return;
  order.splice(index, 0, taskId);
  objectOrders.set(objectId, order);
}

export function insertMockIteration(
  anchor: DemoTask,
  content: IterationContent,
  placement: Placement = "after",
) {
  const fields = taskContent(content);
  if (!fields || !anchor.objectId) return;
  const order = objectIterations(anchor.objectId).map((task) => task.id);
  const index = order.indexOf(anchor.id);
  if (index < 0) return;
  const task: DemoTask = {
    ...fields,
    id: `T-UI-${crypto.randomUUID()}`,
    level: "精修",
    status: "排队中",
    parentId: anchor.parentId,
    objectId: anchor.objectId,
    progress: 0,
    stage: anchor.stage,
  };
  order.splice(index + (placement === "after" ? 1 : 0), 0, task.id);
  objectOrders.set(anchor.objectId, order);
  insertedTasks.add(task.id);
  demoTasks.push(task);
  return task;
}

export function updateMockIteration(task: DemoTask, content: IterationContent) {
  const fields = taskContent(content);
  if (!fields) return;
  if (!insertedTasks.has(task.id) && !originalTasks.has(task.id))
    originalTasks.set(task.id, { ...task });
  Object.assign(task, fields);
  return task;
}

export function resetMockIterations() {
  for (const task of demoTasks) {
    const original = originalTasks.get(task.id);
    if (original) {
      delete task.prompt;
      delete task.annotations;
      Object.assign(task, original);
    }
  }
  originalTasks.clear();
  for (let index = demoTasks.length - 1; index >= 0; index--) {
    if (insertedTasks.has(demoTasks[index]!.id)) demoTasks.splice(index, 1);
  }
  insertedTasks.clear();
  objectOrders.clear();
}
