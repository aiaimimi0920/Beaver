import { z } from "zod";
import type { GameBrief } from "./types";
import { genreChoices, themeChoices } from "./blueprint-catalog";
export const legacyGenreChoices = [{ id: "card", name: "卡牌（已有项目）" }];

const choice = (items: readonly { id: string }[]) =>
  z
    .string()
    .refine((id) => items.some((item) => item.id === id), "请选择目录中的选项");
const weight = () => z.number().int().min(0).max(5);
const capacity = (label: string, min: number, max: number) =>
  z
    .number({ error: `请填写${label}` })
    .int(`${label}必须是整数`)
    .min(min, `${label}不能少于 ${min}`)
    .max(max, `${label}不能超过 ${max}`);
export const onlinePlanSchema = z
  .object({
    enabled: z.boolean(),
    topology: z.enum(["dedicated", "host-relay"]),
    playersPerSession: capacity("每局 / 房间人数", 2, 10000),
    peakCcu: capacity("峰值同时在线人数", 2, 10000000),
    serverCount: capacity("初期实例数", 1, 100000),
    region: z.string().trim().min(1, "请填写目标服务区域").max(120),
  })
  .strict()
  .refine(
    (v) => !v.enabled || v.peakCcu >= v.playersPerSession,
    "峰值同时在线人数不能小于单局人数",
  );
const campaign = () =>
  z
    .object({
      notes: z.string().max(3000),
      channels: z.array(z.string().min(1).max(80)).max(8),
    })
    .strict();
export const blueprintSchema = z
  .object({
    version: z.literal(1),
    genres: z
      .array(choice([...genreChoices, ...legacyGenreChoices]))
      .min(1)
      .max(2)
      .refine((ids) => new Set(ids).size === ids.length, "主副类别不能重复"),
    theme: z.union([
      z
        .object({ mode: z.literal("preset"), value: choice(themeChoices) })
        .strict(),
      z
        .object({
          mode: z.literal("custom"),
          value: z.string().trim().min(1, "请填写自定义主题").max(120),
        })
        .strict(),
    ]),
    audience: z.enum(["all", "young", "mature"]),
    size: z.enum(["indie", "normal", "big", "aaa"]),
    style: z.enum([
      "pixel",
      "illustrated",
      "minimal-2d",
      "low-poly",
      "stylized-3d",
      "text",
    ]),
    online: onlinePlanSchema,
    priorities: z
      .object({
        story: weight(),
        characters: weight(),
        gameplay: weight(),
        graphics: weight(),
        ui: weight(),
        physics: weight(),
        ai: weight(),
        sound: weight(),
        modding: weight(),
        network: weight(),
        animation: weight(),
        optimization: weight(),
        artwork: weight(),
        tutorial: weight(),
        cutscenes: weight(),
      })
      .strict(),
    plannedFeatures: z
      .array(z.string().regex(/^[a-z0-9-]+$/))
      .max(100)
      .refine((ids) => new Set(ids).size === ids.length, "功能规划不能重复"),
    campaigns: z
      .object({ press: campaign(), demo: campaign(), advertising: campaign() })
      .strict(),
  })
  .strict()
  .superRefine((value, ctx) => {
    if (
      value.audience !== "mature" &&
      (value.genres.includes("eroge") ||
        (value.theme.mode === "preset" && value.theme.value === "erotic"))
    )
      ctx.addIssue({
        code: "custom",
        path: ["audience"],
        message: "成人向类别或主题需选择成年人目标评级，或更换类别/主题",
      });
    if (!value.online.enabled && value.genres.includes("mmorpg"))
      ctx.addIssue({
        code: "custom",
        path: ["online"],
        message: "大型多人在线角色扮演需开启在线多人规划，或更换类别",
      });
  });
export type ProjectBlueprint = z.infer<typeof blueprintSchema>;
export type PriorityKey = keyof ProjectBlueprint["priorities"];
export type CampaignKey = keyof ProjectBlueprint["campaigns"];
export interface BlueprintDraft {
  value: ProjectBlueprint;
  revision: number;
}

export function defaultBlueprint(legacy?: GameBrief): ProjectBlueprint {
  const oldGenres: Record<string, string> = {
    narrative: "visual-novel",
    roguelite: "crook-like",
    platformer: "platforms",
  };
  const oldThemes: Record<string, string> = {
    space: "science-fiction",
    modern: "urban",
    historical: "history",
    wasteland: "postapocalyptic",
    campus: "校园青春",
    cozy: "治愈日常",
    nature: "自然生态",
    mystery: "侦探悬疑",
  };
  const translatedGenres = (legacy?.genres ?? ["visual-novel"])
    .map((id) => oldGenres[id] ?? id)
    .filter((id) =>
      [...genreChoices, ...legacyGenreChoices].some((g) => g.id === id),
    );
  const theme = oldThemes[legacy?.theme ?? ""] ?? legacy?.theme ?? "cyberpunk";
  return {
    version: 1,
    genres: translatedGenres.length ? translatedGenres : ["visual-novel"],
    theme: themeChoices.some((t) => t.id === theme)
      ? { mode: "preset", value: theme }
      : { mode: "custom", value: theme },
    audience: "all",
    size: "indie",
    style: (legacy?.style ?? "pixel") as ProjectBlueprint["style"],
    online: {
      enabled: false,
      topology: "dedicated",
      playersPerSession: 8,
      peakCcu: 100,
      serverCount: 1,
      region: "亚洲",
    },
    priorities: {
      story: 3,
      characters: 3,
      gameplay: 3,
      graphics: 3,
      ui: 3,
      physics: 3,
      ai: 3,
      sound: 3,
      modding: 0,
      network: 0,
      animation: 3,
      optimization: 3,
      artwork: 3,
      tutorial: 3,
      cutscenes: 3,
    },
    plannedFeatures: [],
    campaigns: {
      press: { notes: "", channels: [] },
      demo: { notes: "", channels: [] },
      advertising: { notes: "", channels: [] },
    },
  };
}

export function blueprintProblems(value: ProjectBlueprint): string[] {
  const result = blueprintSchema.safeParse(value);
  return result.success
    ? []
    : [...new Set(result.error.issues.map((issue) => issue.message))];
}
export function suggestedBlueprintFeatures(value: ProjectBlueprint): string[] {
  const groups = genreChoices
    .filter((g) => value.genres.includes(g.id))
    .map((g) => g.group);
  return [
    ...new Set([
      "save-slot",
      ...(groups.includes("叙事")
        ? ["dialogue", "relationship", "quests", "localization"]
        : []),
      ...(groups.includes("角色扮演")
        ? ["inventory", "quests", "progression"]
        : []),
      ...(groups.includes("模拟经营") || groups.includes("策略")
        ? ["economy", "inventory", "day-cycle"]
        : []),
    ]),
  ];
}
