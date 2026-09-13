import { z } from "zod";
import { directionSchema } from "./task-board";

export const taskPlanSchema = z
  .object({
    summary: z.string().trim().min(1).max(2000),
    steps: z
      .array(
        z
          .object({
            title: z.string().trim().min(1).max(120),
            prompt: z.string().trim().min(1).max(6000),
            direction: directionSchema,
            acceptance: z.string().trim().min(1).max(2000),
          })
          .strict(),
      )
      .min(2)
      .max(12),
  })
  .strict()
  .refine(
    (plan) =>
      new Set(plan.steps.map((s) => s.title)).size === plan.steps.length,
    "子任务标题不能重复",
  );

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
