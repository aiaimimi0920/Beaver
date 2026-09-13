import { z } from "zod";

export const questionsSchema = z
  .object({
    questions: z
      .array(
        z.object({
          id: z.string().min(1).max(80),
          question: z.string().trim().min(1).max(3000),
          isSecret: z.literal(false).optional(),
          importance: z.number().int().min(1).max(100).optional(),
          recommended: z.string().min(1).max(200).optional(),
          reason: z.string().min(1).max(1000).optional(),
          options: z
            .array(
              z.object({
                label: z.string().min(1).max(200),
                description: z.string().max(1000).optional(),
              }),
            )
            .max(8)
            .nullable()
            .optional(),
        }),
      )
      .min(1)
      .max(3),
  })
  .refine(
    ({ questions }) =>
      new Set(questions.map((q) => q.id)).size === questions.length,
    "问题标识重复",
  );

export interface Clarification {
  id: string;
  createdAt: string;
  questions: z.infer<typeof questionsSchema>["questions"];
  answers?: Record<string, string>;
  answeredAt?: string;
  automaticQuestions?: Clarification["questions"];
  autoAnswers?: Record<string, string>;
  askRatio?: number;
  appliedAskRatio?: number;
  answerSources?: Record<string, "automatic" | "user">;
}

export function validateAnswers(
  item: Clarification,
  input: unknown,
): Record<string, string> {
  if (item.answers) throw new Error("问题已经回答");
  const answers = z
    .record(z.string(), z.string().trim().min(1).max(10000))
    .parse(input);
  if (
    Object.keys(answers).length !== item.questions.length ||
    item.questions.some((q) => !Object.hasOwn(answers, q.id))
  )
    throw new Error("请回答全部问题");
  return answers;
}

export const askUserTool = {
  type: "function",
  name: "beaver_ask_user",
  description:
    "Ask the user for missing creative intent or project knowledge. This parks the task until the user answers. Ask one decision per question, 1-3 questions per round. Every question must provide 2-6 distinct, short, selectable options in the structured options field, with descriptions explaining meaningful tradeoffs. Do not embed A/B/C choices only in question text. The UI always permits a custom answer, so do not add an Other/custom option. For uncertain references offer concrete interpretations and allow the user to clarify. Read existing project documents first; do not invent user decisions or repeat answered questions. Never request passwords or credentials.",
  inputSchema: {
    type: "object",
    additionalProperties: false,
    required: ["questions"],
    properties: {
      questions: {
        type: "array",
        minItems: 1,
        maxItems: 3,
        items: {
          type: "object",
          additionalProperties: false,
          required: [
            "id",
            "question",
            "options",
            "importance",
            "recommended",
            "reason",
          ],
          properties: {
            id: { type: "string", minLength: 1, maxLength: 80 },
            question: { type: "string", minLength: 1, maxLength: 3000 },
            importance: {
              type: "integer",
              minimum: 1,
              maximum: 100,
              description:
                "Decision impact, independent of automation preference: 91-100 core identity/high rework risk; 71-90 major creative direction; 31-70 ordinary design decisions; 1-30 minor reversible details. Do not inflate scores to force user interaction.",
            },
            recommended: {
              type: "string",
              description:
                "Exact label of the recommended option. Respect all already confirmed user choices.",
            },
            reason: {
              type: "string",
              minLength: 1,
              maxLength: 1000,
              description:
                "Why this option fits the user's goal. Beaver decides whether to ask the user or apply it automatically.",
            },
            options: {
              type: "array",
              minItems: 2,
              maxItems: 6,
              items: {
                type: "object",
                additionalProperties: false,
                required: ["label", "description"],
                properties: {
                  label: { type: "string", minLength: 1, maxLength: 200 },
                  description: { type: "string", maxLength: 1000 },
                },
              },
            },
          },
        },
      },
    },
  },
};
