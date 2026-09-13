import { z } from "zod";
import type { Project } from "./types";
import {
  blueprintSchema,
  defaultBlueprint,
  type ProjectBlueprint,
} from "./project-blueprint";

export const overviewFields = {
  name: "游戏名称",
  genres: "主副类别",
  theme: "游戏主题",
  audience: "目标评级",
  size: "游戏规模",
  style: "表现风格",
  online: "在线多人设定",
} as const;

export const overviewSchema = z
  .object({
    name: z
      .string()
      .trim()
      .min(1, "请填写游戏名称")
      .max(80, "游戏名称不能超过 80 个字符")
      .regex(/^[^\x00-\x1f]+$/, "游戏名称不能包含控制字符"),
    genres: blueprintSchema.shape.genres,
    theme: blueprintSchema.shape.theme,
    audience: blueprintSchema.shape.audience,
    size: blueprintSchema.shape.size,
    style: blueprintSchema.shape.style,
    online: blueprintSchema.shape.online,
  })
  .strict();
export type ProjectOverview = z.infer<typeof overviewSchema>;
export interface OverviewDraft {
  value: ProjectOverview;
  revision: number;
}

export function overviewFromPlan(
  name: string,
  plan: ProjectBlueprint,
): ProjectOverview {
  return {
    name,
    genres: plan.genres,
    theme: plan.theme,
    audience: plan.audience,
    size: plan.size,
    style: plan.style,
    online: plan.online,
  };
}
export function projectOverview(project: Project): ProjectOverview {
  return overviewFromPlan(
    project.name,
    project.blueprint ?? defaultBlueprint(project.design),
  );
}
export function changedOverviewFields(
  before: ProjectOverview,
  after: ProjectOverview,
): string[] {
  return (Object.keys(overviewFields) as (keyof ProjectOverview)[])
    .filter((key) => JSON.stringify(before[key]) !== JSON.stringify(after[key]))
    .map((key) => overviewFields[key]);
}
export function overviewBlueprint(
  value: ProjectOverview,
  base: ProjectBlueprint,
): ProjectBlueprint {
  const { name: _name, ...settings } = value;
  return { ...base, ...settings };
}
