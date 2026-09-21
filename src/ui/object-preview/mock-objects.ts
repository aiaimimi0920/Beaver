export type PreviewPage = "tasks" | "objects" | "creation";
import type { ObjectType } from "./object-categories";
export type { ObjectType } from "./object-categories";
export type ArtKind =
  "character" | "classroom" | "performance" | "code" | "texture" | "ui";
export type DemoStatus =
  | "执行中"
  | "待验收"
  | "排队中"
  | "等待依赖"
  | "已验收"
  | "未绑定任务"
  | "执行失败";
export interface DemoObject {
  id: string;
  name: string;
  kind: ArtKind;
  category: string;
  objectType: ObjectType;
  tags: string[];
  description: string;
  status: DemoStatus;
  version: string;
  accepted: string | null;
  taskId: string | null;
  components: { name: string; format: string; objectType: ObjectType }[];
  references: { objectId: string; version: string }[];
  modified: string;
}

export const demoObjects: DemoObject[] = [
  {
    id: "O-001",
    name: "澪 · NPR 角色",
    kind: "character",
    category: "角色",
    objectType: "模型",
    tags: ["NPR", "角色", "校园"],
    description: "青蓝短发 / 学院制服 / 三段式卡通阴影",
    status: "待验收",
    version: "r04",
    accepted: "r03",
    taskId: "T-101",
    components: [
      { name: "角色模型", format: "BLEND · GLB", objectType: "模型" },
      { name: "基础色贴图", format: "PNG · 2048²", objectType: "图像" },
      { name: "法线贴图", format: "PNG · 2048²", objectType: "图像" },
      { name: "NPR 材质", format: "SHADER", objectType: "脚本" },
    ],
    references: [],
    modified: "刚刚",
  },
  {
    id: "O-002",
    name: "午后教室",
    kind: "classroom",
    category: "场景",
    objectType: "场景",
    tags: ["校园", "环境"],
    description: "木质课桌 / 大面积窗光 / 可探索空间",
    status: "执行中",
    version: "r02",
    accepted: "r01",
    taskId: "T-102",
    components: [
      { name: "教室场景", format: "TSCN", objectType: "场景" },
      { name: "课桌与椅子", format: "GLB", objectType: "模型" },
      { name: "环境材质", format: "TRES", objectType: "其他" },
    ],
    references: [{ objectId: "O-005", version: "r01" }],
    modified: "2 分钟前",
  },
  {
    id: "O-003",
    name: "放课后的舞步",
    kind: "performance",
    category: "场景",
    objectType: "场景",
    tags: ["校园", "演出"],
    description: "角色演出 / 镜头调度 / 教室内舞蹈",
    status: "等待依赖",
    version: "r01",
    accepted: null,
    taskId: "T-103",
    components: [
      { name: "演出场景", format: "TSCN", objectType: "场景" },
      { name: "舞蹈动作", format: "ANIMATION", objectType: "其他" },
      { name: "镜头轨道", format: "TRES", objectType: "其他" },
    ],
    references: [
      { objectId: "O-001", version: "r03" },
      { objectId: "O-002", version: "r01" },
    ],
    modified: "8 分钟前",
  },
  {
    id: "O-004",
    name: "舞蹈控制器",
    kind: "code",
    category: "代码",
    objectType: "脚本",
    tags: ["演出", "交互"],
    description: "播放 / 暂停 / 动作过渡与节拍同步",
    status: "已验收",
    version: "r02",
    accepted: "r02",
    taskId: "T-104",
    components: [
      { name: "播放控制逻辑", format: "GDSCRIPT", objectType: "脚本" },
      { name: "控制器场景", format: "TSCN", objectType: "场景" },
    ],
    references: [],
    modified: "16 分钟前",
  },
  {
    id: "O-005",
    name: "浅色橡木",
    kind: "texture",
    category: "贴图",
    objectType: "图像",
    tags: ["环境", "可复用"],
    description: "可平铺木纹 / 独立对象 / 环境通用",
    status: "未绑定任务",
    version: "r01",
    accepted: "r01",
    taskId: null,
    components: [
      { name: "木纹基础色", format: "PNG · 2048²", objectType: "图像" },
      { name: "表面法线", format: "PNG · 2048²", objectType: "图像" },
    ],
    references: [],
    modified: "昨天",
  },
  {
    id: "O-006",
    name: "演出控制界面",
    kind: "ui",
    category: "界面",
    objectType: "场景",
    tags: ["交互", "可复用"],
    description: "轻量播放器 / 进度条 / 快捷按键提示",
    status: "未绑定任务",
    version: "r01",
    accepted: "r01",
    taskId: null,
    components: [
      { name: "播放器根场景", format: "TSCN", objectType: "场景" },
      { name: "播放按钮", format: "子对象 · CONTROL", objectType: "场景" },
      { name: "进度条", format: "子对象 · CONTROL", objectType: "场景" },
    ],
    references: [{ objectId: "O-004", version: "r02" }],
    modified: "昨天",
  },
];

export const objectById = (id: string) =>
  demoObjects.find((object) => object.id === id) ?? demoObjects[0]!;
export function statusTone(status: DemoStatus): string {
  if (status === "待验收") return "attention";
  if (status === "已验收") return "success";
  if (status === "执行中") return "info";
  if (status === "执行失败") return "danger";
  return "muted";
}
