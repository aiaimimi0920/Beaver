import test from "node:test";
import assert from "node:assert/strict";
import {
  askUserTool,
  questionsSchema,
  validateAnswers,
} from "../src/shared/clarifications";

test("Codex tool advertises structured choices and preserves custom answers", () => {
  const advertised = askUserTool.inputSchema.properties.questions.items;
  assert(advertised.required.includes("options"));
  assert.equal(advertised.properties.options.minItems, 2);
  const questions = questionsSchema.parse({
    questions: [
      {
        id: "tone",
        question: "角色的语气？",
        options: [
          { label: "温柔", description: "重视共情" },
          { label: "冷峻", description: "突出疏离" },
        ],
      },
    ],
  }).questions;
  assert.equal(questions[0]?.options?.[1]?.description, "突出疏离");
  const item = { id: "batch", createdAt: "now", questions };
  assert.deepEqual(validateAnswers(item, { tone: "冷峻" }), { tone: "冷峻" });
  assert.deepEqual(validateAnswers(item, { tone: "  外冷内热  " }), {
    tone: "外冷内热",
  });
});

test("creative questions reject secrets, duplicates and excessive batches", () => {
  const q = { id: "tone", question: "角色的语气？" };
  assert.equal(questionsSchema.safeParse({ questions: [q] }).success, true);
  for (const questions of [
    [],
    [q, q],
    [{ ...q, isSecret: true }],
    [q, q, q, q],
  ])
    assert.equal(questionsSchema.safeParse({ questions }).success, false);
});

test("answers require exact question IDs and nonempty values; replay is rejected", () => {
  const item = {
    id: "batch",
    createdAt: "now",
    questions: [{ id: "tone", question: "语气？" }],
  };
  assert.deepEqual(validateAnswers(item, { tone: "  温柔  " }), {
    tone: "温柔",
  });
  for (const value of [
    {},
    { wrong: "a" },
    { tone: " " },
    { tone: "a", extra: "b" },
  ])
    assert.throws(() => validateAnswers(item, value));
  assert.throws(() =>
    validateAnswers({ ...item, answers: { tone: "a" } }, { tone: "b" }),
  );
});
