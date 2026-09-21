import type { DemoStatus } from "./mock-objects";
import type { PreviewAnnotation } from "./preview-target";

export interface DemoTask {
  id: string;
  title: string;
  summary?: string;
  level: "粗修" | "中修" | "精修";
  status: DemoStatus;
  parentId: string | null;
  objectId: string | null;
  detail: string;
  prompt?: string;
  progress: number;
  stage?: number;
  annotations?: PreviewAnnotation[];
}

export const demoTasks: DemoTask[] = [
  {
    id: "T-100",
    title: "让澪在午后的教室里跳舞",
    summary: "教室舞蹈场景",
    level: "粗修",
    status: "执行中",
    parentId: null,
    objectId: null,
    detail: "完成角色、教室与舞蹈演出，交付可在 Godot 中播放的场景。",
    progress: 58,
  },
  {
    id: "T-101",
    title: "制作澪的 NPR 角色",
    summary: "制作 NPR 角色",
    level: "中修",
    status: "待验收",
    parentId: "T-100",
    objectId: "O-001",
    detail: "青蓝短发、学院制服，优先保证面部与头发的卡通表现。",
    progress: 50,
  },
  {
    id: "T-102",
    title: "搭建午后教室",
    summary: "搭建午后教室",
    level: "中修",
    status: "执行中",
    parentId: "T-100",
    objectId: "O-002",
    detail: "暖色窗光，课桌靠两侧排列，为角色留出演出空间。",
    progress: 42,
  },
  {
    id: "T-103",
    title: "编排教室舞蹈演出",
    summary: "编排舞蹈演出",
    level: "中修",
    status: "等待依赖",
    parentId: "T-100",
    objectId: "O-003",
    detail: "等待角色与教室最终验收。采用明确的已验收版本后继续。",
    progress: 8,
  },
  {
    id: "T-104",
    title: "实现舞蹈播放控制",
    summary: "舞蹈播放控制",
    level: "中修",
    status: "已验收",
    parentId: "T-100",
    objectId: "O-004",
    detail: "播放、暂停、节拍同步与动作平滑过渡。",
    progress: 100,
  },
  {
    id: "T-201",
    title: "调整面部阴影与发丝高光",
    summary: "阴影与发丝高光",
    level: "精修",
    status: "待验收",
    parentId: "T-101",
    objectId: "O-001",
    detail: "r04 的表面与风格阶段交付，等待确认后进入下一阶段。",
    progress: 100,
    stage: 2,
  },
  {
    id: "T-202",
    title: "优化窗光与室内明暗",
    summary: "窗光与室内明暗",
    level: "精修",
    status: "执行中",
    parentId: "T-102",
    objectId: "O-002",
    detail: "调整教室窗侧亮度，保留桌椅阴影的清晰层次。",
    progress: 65,
    stage: 2,
  },
  {
    id: "T-203",
    title: "完成角色形体与拓扑",
    summary: "角色形体与拓扑",
    level: "精修",
    status: "已验收",
    parentId: "T-101",
    objectId: "O-001",
    detail: "基础结构已确认，作为后续材质与绑定的输入。",
    progress: 100,
    stage: 1,
  },
  {
    id: "T-204",
    title: "检查环境贴图导入",
    summary: "环境贴图导入",
    level: "精修",
    status: "执行失败",
    parentId: "T-102",
    objectId: "O-002",
    detail: "示例错误：一张法线贴图使用了错误的色彩空间。等待修正。",
    progress: 70,
    stage: 2,
  },
  {
    id: "T-105",
    title: "为澪增加蓝色发饰",
    summary: "增加蓝色发饰",
    level: "中修",
    status: "排队中",
    parentId: null,
    objectId: "O-001",
    detail: "队列第 1 位。T-101 完成最终验收后，从角色最新已验收版本开始。",
    progress: 0,
  },
];

export const stageNames = [
  "需求与设定",
  "形体与结构",
  "表面与风格",
  "绑定与动作",
  "引擎整合",
  "对象验收",
];
export const stageDeliveries = [
  ["角色设定与比例", "外观参考板", "制作规格"],
  ["角色基础模型", "结构与拓扑检查", "三视图"],
  ["NPR 材质预览", "基础色与法线贴图", "阴影与高光对照"],
  ["骨骼与权重", "舞蹈测试动作", "变形检查"],
  ["Godot 对象场景", "材质与动画导入", "引擎运行截图"],
  ["最终对象包", "验收清单", "版本与依赖快照"],
];

export const taskById = (id: string) =>
  demoTasks.find((task) => task.id === id) ?? demoTasks[0]!;
