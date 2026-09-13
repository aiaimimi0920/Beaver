import { z } from "zod";
import type { GameBrief, Feature } from "./types";

export const genres = [
  {
    id: "narrative",
    name: "叙事 / 视觉小说",
    features: ["dialogue", "relationship", "quests", "localization"],
  },
  {
    id: "rpg",
    name: "角色扮演",
    features: ["inventory", "quests", "progression", "dialogue"],
  },
  {
    id: "adventure",
    name: "探索冒险",
    features: ["inventory", "quests", "dialogue"],
  },
  {
    id: "simulation",
    name: "生活模拟",
    features: ["day-cycle", "relationship", "economy"],
  },
  {
    id: "management",
    name: "经营管理",
    features: ["economy", "inventory", "day-cycle"],
  },
  {
    id: "strategy",
    name: "策略",
    features: ["economy", "progression", "day-cycle"],
  },
  { id: "puzzle", name: "解谜", features: ["quests", "inventory"] },
  {
    id: "survival",
    name: "生存制作",
    features: ["inventory", "crafting", "day-cycle"],
  },
  { id: "action", name: "动作", features: ["progression", "inventory"] },
  { id: "platformer", name: "平台跳跃", features: ["progression", "quests"] },
  {
    id: "roguelite",
    name: "Roguelite",
    features: ["inventory", "progression", "economy"],
  },
  {
    id: "card",
    name: "卡牌",
    features: ["inventory", "progression", "economy"],
  },
] as const;
export const themes = [
  { id: "cyberpunk", name: "赛博朋克" },
  { id: "space", name: "太空探索" },
  { id: "fantasy", name: "奇幻" },
  { id: "modern", name: "现代都市" },
  { id: "historical", name: "历史" },
  { id: "wasteland", name: "废土末日" },
  { id: "mystery", name: "侦探悬疑" },
  { id: "horror", name: "怪谈惊悚" },
  { id: "campus", name: "校园青春" },
  { id: "cozy", name: "治愈日常" },
  { id: "ocean", name: "航海" },
  { id: "nature", name: "自然生态" },
] as const;
export const styles = [
  { id: "pixel", name: "像素 2D" },
  { id: "illustrated", name: "手绘 2D" },
  { id: "minimal-2d", name: "简约 2D" },
  { id: "low-poly", name: "低多边形 3D" },
  { id: "stylized-3d", name: "风格化 3D" },
  { id: "text", name: "文字为主" },
] as const;
export const scopes = [
  {
    id: "prototype",
    name: "可玩原型",
    goal: "先做 5–10 分钟可体验的核心循环，优先用占位素材验证玩法。",
  },
  {
    id: "vertical-slice",
    name: "精做一章",
    goal: "完成一个 15–30 分钟的代表性章节，验证玩法、美术和音频的统一效果。",
  },
  {
    id: "short-game",
    name: "短篇游戏",
    goal: "目标为约 30–60 分钟的短篇体验，包含开场、完整核心循环和结局；不要无限扩大内容。",
  },
] as const;
export const featureCategories = [
  { id: "narrative", name: "叙事" },
  { id: "systems", name: "玩法系统" },
  { id: "progression", name: "成长" },
  { id: "foundation", name: "基础能力" },
] as const;

const choice = (items: readonly { id: string }[]) =>
  z
    .string()
    .refine((id) => items.some((item) => item.id === id), "目录选项不存在");
export const gameBriefSchema = z
  .object({
    genres: z
      .array(choice(genres))
      .min(1)
      .max(2)
      .refine((ids) => new Set(ids).size === ids.length, "主副类型不能重复"),
    theme: choice(themes),
    style: choice(styles),
    scope: choice(scopes),
  })
  .strict();

export const defaultGameBrief = (): GameBrief => ({
  genres: ["narrative"],
  theme: "cyberpunk",
  style: "pixel",
  scope: "prototype",
});

export function recommendedFeatures(design?: GameBrief): string[] {
  if (!design) return [];
  return [
    ...new Set([
      "save-slot",
      ...genres
        .filter((g) => design.genres.includes(g.id))
        .flatMap((g) => [...g.features]),
    ]),
  ];
}

export function formatGameBrief(design?: GameBrief): string {
  if (!design) return "";
  return [
    "项目创作方向（不是已实现功能；本次明确要求优先，不自行接入未要求的功能块）：",
    `类型：${design.genres.map((id) => genres.find((g) => g.id === id)?.name ?? id).join(" + ")}`,
    `题材：${themes.find((t) => t.id === design.theme)?.name}`,
    `表现：${styles.find((s) => s.id === design.style)?.name}`,
    `规模：${scopes.find((s) => s.id === design.scope)?.goal}`,
    "剧情、角色、图像、音乐与代码均采用原创或有权使用的内容。不要把类型选择误当成完整游戏模板已经具备相关玩法。",
  ].join("\n");
}

export const featureSchema = z
  .object({
    id: z.string().regex(/^[a-z0-9-]+$/),
    name: z.string().min(1),
    version: z.string().regex(/^\d+\.\d+\.\d+$/),
    description: z.string().min(1),
    category: choice(featureCategories).optional(),
    kind: z.enum(["module", "reference"]).optional(),
  })
  .strict();

export function featureCategory(feature: Feature): string {
  return (
    feature.category ?? (feature.id === "dialogue" ? "narrative" : "foundation")
  );
}
