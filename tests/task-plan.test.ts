import test from "node:test";
import assert from "node:assert/strict";
import { askUserTool } from "../src/shared/clarifications";
import {
  planTool,
  taskPlanSchema,
  nativeTaskPlanSchema,
} from "../src/shared/task-plan";

test("planning and clarification tools use the same canonical app-server format", () => {
  assert.equal(planTool.type, "function");
  assert.equal(planTool.type, askUserTool.type);
  assert.equal(planTool.name, "beaver_submit_plan");
  assert.equal(planTool.inputSchema.type, "object");
  assert.equal(planTool.inputSchema.additionalProperties, false);
});

test("workflow selection is explicit, bounded and never defaults all steps to NPR", () => {
  const step = {
    title: "room",
    prompt: "empty room",
    direction: "visual",
    acceptance: "room visible",
    workflow: "general",
  };
  const npr = {
    ...step,
    title: "character",
    prompt: "explicit NPR character",
    workflow: "npr-character",
  };
  assert.equal(
    taskPlanSchema.safeParse({ summary: "native-only", steps: [step, npr] })
      .success,
    false,
  );
  const result = nativeTaskPlanSchema.parse({
    summary: "mixed workflow",
    steps: [step, npr],
  });
  assert.ok(result.steps[0]);
  assert.ok(result.steps[1]);
  assert.equal(result.steps[0].workflow, "general");
  assert.equal(result.steps[1].workflow, "npr-character");
  assert.equal(
    nativeTaskPlanSchema.safeParse({
      summary: "invalid",
      steps: [step, { ...npr, workflow: "unknown" }],
    }).success,
    false,
  );
  assert.equal(
    nativeTaskPlanSchema.safeParse({
      summary: "partial",
      steps: [step, { ...npr, workflow: undefined }],
    }).success,
    false,
  );
});
test("plans require distinct executable steps and explicit acceptance criteria", () => {
  const step = {
    title: "实现",
    prompt: "独立目标",
    direction: "engineering",
    acceptance: "验证结果",
  };
  assert.equal(
    taskPlanSchema.safeParse({ summary: "规划", steps: [step] }).success,
    false,
  );
  assert.equal(
    taskPlanSchema.safeParse({ summary: "规划", steps: [step, step] }).success,
    false,
  );
  assert.equal(
    taskPlanSchema.safeParse({
      summary: "规划",
      steps: [step, { ...step, title: "验证", acceptance: "" }],
    }).success,
    false,
  );
  assert.equal(
    taskPlanSchema.safeParse({
      summary: "规划",
      steps: [step, { ...step, title: "验证" }],
    }).success,
    true,
  );
});
