import { z } from "zod";

export const previewStatus = [
  "执行中",
  "正在整合",
  "待验收",
  "需要你决定",
  "已暂停",
  "已认可",
  "已回退",
] as const;
const noteSchema = z.object({
  id: z.string(),
  name: z.string(),
  folder: z.string(),
  text: z.string(),
  draft: z.string().optional(),
});
const assetSchema = z.object({
  id: z.string(),
  name: z.string(),
  folder: z.string(),
  type: z.enum(["图片", "音频", "模型", "场景", "翻译"]),
  tags: z.array(z.string()),
  notes: z.string(),
  favorite: z.boolean(),
});
const taskSchema = z.object({
  id: z.string(),
  goal: z.string(),
  status: z.enum(previewStatus),
  pausedFrom: z.enum(previewStatus).optional(),
  refs: z.array(z.string()),
  messages: z.array(z.string()),
  conflict: z.enum(["script", "binary"]).optional(),
  kept: z.array(z.string()).optional(),
});
export const studioSchema = z.object({
  version: z.literal(1),
  notes: z.array(noteSchema).min(1),
  assets: z.array(assetSchema),
  tasks: z.array(taskSchema),
});
export type StudioState = z.infer<typeof studioSchema>;
export type StudioNote = z.infer<typeof noteSchema>;
export type StudioAsset = z.infer<typeof assetSchema>;
export type StudioTask = z.infer<typeof taskSchema>;
export type StudioPage = "create" | "docs" | "library";
export function pausePreviewTask(task: StudioTask): StudioTask {
  return { ...task, pausedFrom: task.status, status: "已暂停" };
}
export function resumePreviewTask(task: StudioTask): StudioTask {
  return {
    ...task,
    status: task.pausedFrom ?? (task.conflict ? "需要你决定" : "执行中"),
    pausedFrom: undefined,
  };
}
export function createStudioState(): StudioState {
  return {
    version: 1,
    notes: [
      {
        id: "direction",
        name: "创作约定",
        folder: "方向与约定",
        text: "# 创作约定\n\n用调饮和对话讲述普通人的夜晚。\n\n## 核心体验\n让玩家通过倾听与选择了解客人，不以操作速度制造难度。",
      },
      {
        id: "world",
        name: "城市与酒吧",
        folder: "世界与故事",
        text: "# 城市与酒吧\n\n夜航是一间沿海城市的小酒吧。雨季里，客人常在这里等待末班车。\n\n## 常客\n[[阿澄 · 人物介绍]]常在夜班结束后到店休息。",
      },
      {
        id: "character",
        name: "阿澄 · 人物介绍",
        folder: "人物与关系",
        text: "# 阿澄 · 人物介绍\n\n## 身份与动机\n社区配送员。正在攒钱离开城市，却舍不得熟悉的人。\n\n## 说话方式\n直接，偶尔开玩笑；谈到过去时会停顿，不会突然长篇解释。\n\n## 人物关系\n与老板是多年的朋友。老板知道她在攒钱，但从不替她做决定。\n\n## 视觉约定\n旧款防雨外套，低饱和配色。表情变化优先于大量身体动作。\n\n## 相关资料\n[[城市与酒吧]] · [[角色视觉规范]]",
      },
      {
        id: "story",
        name: "第一晚剧情",
        folder: "世界与故事",
        text: "# 第一晚剧情\n\n开店 → 第一位客人 → 学习调饮 → 第二位客人 → 打烊。\n\n## 选择\n玩家可以追问，也可以安静地递上一杯饮料。",
      },
      {
        id: "rules",
        name: "调饮规则",
        folder: "玩法与系统",
        text: "# 调饮规则\n\n选择原料、调制方式和饮品，然后交给客人。\n\n## 反馈\n错误饮品可以改变对话，但不应直接终止故事。",
      },
      {
        id: "scene",
        name: "酒吧空间",
        folder: "关卡与场景",
        text: "# 酒吧空间\n\n可交互区域：座位、调饮台、收音机和门口。\n\n## 镜头\n固定机位，避免隐藏重要交互。",
      },
      {
        id: "art",
        name: "角色视觉规范",
        folder: "美术与声音",
        text: "# 角色视觉规范\n\n## 比例\n统一头身比、描边粗细和光照方向。\n\n## 色彩\n低饱和背景，重要表情清晰可辨。",
      },
      {
        id: "tech",
        name: "存档与验收",
        folder: "技术与测试",
        text: "# 存档与验收\n\n## 存档\n在对话段落之间自动保存。\n\n## 验收\n继续游戏后应保留饮品选择和对话进度。",
      },
      {
        id: "release",
        name: "本地化与交付",
        folder: "发行与本地化",
        text: "# 本地化与交付\n\n## 语言\n简体中文，预留英文文本。\n\n## 交付\n可直接启动的 Windows 游戏程序。",
      },
    ],
    assets: [
      {
        id: "cheng",
        name: "阿澄 · 立绘",
        folder: "人物/阿澄",
        type: "图片",
        tags: ["像素风", "待调整"],
        notes: "保留防雨外套，降低肩部亮度。",
        favorite: true,
      },
      {
        id: "yuan",
        name: "远舟 · 立绘",
        folder: "人物/远舟",
        type: "图片",
        tags: ["像素风"],
        notes: "",
        favorite: false,
      },
      {
        id: "boss",
        name: "老板 · 立绘",
        folder: "人物/老板",
        type: "图片",
        tags: ["像素风"],
        notes: "",
        favorite: false,
      },
      {
        id: "bar",
        name: "雨夜酒吧 · 背景",
        folder: "场景/夜航酒吧",
        type: "图片",
        tags: ["雨夜"],
        notes: "",
        favorite: true,
      },
      {
        id: "music",
        name: "雨夜 · 环境音乐",
        folder: "声音/音乐",
        type: "音频",
        tags: ["舒缓"],
        notes: "",
        favorite: false,
      },
      {
        id: "sound",
        name: "调饮 · 杯具音效",
        folder: "声音/音效",
        type: "音频",
        tags: [],
        notes: "",
        favorite: false,
      },
      {
        id: "model",
        name: "吧台 · 模型",
        folder: "场景/夜航酒吧",
        type: "模型",
        tags: [],
        notes: "",
        favorite: false,
      },
      {
        id: "scene",
        name: "酒吧 · 场景",
        folder: "场景/夜航酒吧",
        type: "场景",
        tags: [],
        notes: "",
        favorite: false,
      },
      {
        id: "translation",
        name: "第一晚 · 中英对照",
        folder: "本地化",
        type: "翻译",
        tags: [],
        notes: "",
        favorite: false,
      },
    ],
    tasks: [
      {
        id: "save",
        goal: "增加自动存档",
        status: "正在整合",
        refs: [],
        messages: [],
      },
      {
        id: "load",
        goal: "增加读取存档界面",
        status: "需要你决定",
        refs: ["scripts/save.gd", "docs/存档与验收.md"],
        messages: [],
        conflict: "script",
      },
      {
        id: "look",
        goal: "调整阿澄的外套和表情",
        status: "需要你决定",
        refs: ["阿澄 · 立绘"],
        messages: [],
        conflict: "binary",
      },
      {
        id: "style",
        goal: "统一三位客人的视觉风格",
        status: "待验收",
        refs: ["阿澄 · 立绘", "远舟 · 立绘", "老板 · 立绘"],
        messages: [],
      },
      {
        id: "night",
        goal: "完成第一晚的调饮与对话",
        status: "执行中",
        refs: [],
        messages: [],
      },
    ],
  };
}
export function nextPreviewStatus(
  status: StudioTask["status"],
): StudioTask["status"] {
  return status === "执行中"
    ? "正在整合"
    : status === "正在整合"
      ? "待验收"
      : status;
}
