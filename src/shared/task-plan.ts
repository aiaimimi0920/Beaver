import { z } from "zod";
import { directionSchema } from "./task-board";

const stepSchema = z
  .object({
    title: z.string().trim().min(1).max(120),
    prompt: z.string().trim().min(1).max(6000),
    direction: directionSchema,
    acceptance: z.string().trim().min(1).max(2000),
  })
  .strict();
const planShape = z
  .object({
    summary: z.string().trim().min(1).max(2000),
    steps: z.array(stepSchema).min(2).max(12),
  })
  .strict();
const distinctTitles = (plan: { steps: { title: string }[] }) =>
  new Set(plan.steps.map((step) => step.title)).size === plan.steps.length;

// Electron retains its existing plan format and cannot silently discard native selection.
export const taskPlanSchema = planShape.refine(
  distinctTitles,
  "子任务标题不能重复",
);
export const nativeTaskPlanSchema = planShape
  .extend({
    steps: z
      .array(
        stepSchema
          .extend({
            workflow: z
              .enum(["general", "npr-character"])
              .optional()
              .describe(
                "Choose from discovered workflows. Use npr-character only for an explicit NPR character goal; general preserves other creation workflows. Select a value for every step in a new plan.",
              ),
          })
          .strict(),
      )
      .min(2)
      .max(12),
  })
  .refine(
    (plan) =>
      plan.steps.every((step) => step.workflow === undefined) ||
      plan.steps.every((step) => step.workflow !== undefined),
    "每个步骤都应选择工作流；旧格式可全部省略",
  )
  .refine(distinctTitles, "子任务标题不能重复");

export type TaskPlan = z.infer<typeof taskPlanSchema>;
export const planTool = {
  type: "function",
  name: "beaver_submit_plan",
  description:
    "Submit 2-12 ordered executable child tasks after clarifying the user's goal. This ends planning; Beaver creates, runs and approves each child separately. Do not implement the whole goal in the parent task.",
  inputSchema: z.toJSONSchema(taskPlanSchema),
};
export const planningInstruction =
  "本任务采用拆分执行。你现在负责理解目标和规划，不在父任务中完成整个游戏。先用 beaver_ask_user 追问必要细节，每题提供推荐选项并允许自定义，按自动决策策略处理。确认目标后调用 beaver_submit_plan，提交 2-12 个按执行顺序排列的具体子任务，每项包含标题、方向、独立目标、可验证的验收标准。拆分应根据用户实际需求，覆盖实现和最终验证；不要仅提交文档计划或固定分类。可以保存确认记录，但代码、素材与运行验证由后续子任务执行。提交计划后本阶段停止，由 Beaver 逐项执行；后续任务会获得之前已合入的成果。";

export const nativePlanTool = {
  ...planTool,
  inputSchema: z.toJSONSchema(nativeTaskPlanSchema),
};
export const nativePlanningInstruction =
  planningInstruction +
  "先通过 beaver_workflow_list 发现可用工作流；只有用户明确要求NPR角色时才为相关制作/验证步骤选择 npr-character，其他步骤选择 general，每一步填写 workflow。选择会冻结源版本到子任务并在执行前核验，不可把未启用工作流当作可用。";
