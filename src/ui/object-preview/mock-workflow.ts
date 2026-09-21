import type { DemoObject, ObjectType } from "./mock-objects";
import type { DemoTask } from "./mock-tasks";

export interface WorkflowStep {
  title: string;
  skill: string;
  calls: string[];
  instruction: string;
  inputs: string[];
  outputs: string[];
  acceptance: string;
}

interface WorkflowRecipe {
  skill: string;
  calls: string[];
  output: string;
  preview: string;
}

const recipes: Record<ObjectType, WorkflowRecipe> = {
  模型: {
    skill: "model-edit",
    calls: [
      "Blender MCP · 打开模型与关联材质",
      "Blender 插件 · 编辑目标区域",
      "Blender · 导出 GLB 与贴图",
    ],
    output: "模型源文件、GLB、关联贴图与材质",
    preview: "Godot MCP · 导入模型，在对象场景中渲染多角度画面",
  },
  场景: {
    skill: "scene-edit",
    calls: [
      "Godot MCP · 读取场景节点与资源引用",
      "Godot 插件 · 调整目标节点与场景参数",
      "Godot · 保存场景与资源",
    ],
    output: "场景文件与变更的关联资源",
    preview: "Godot MCP · 运行场景，捕获指定镜头画面",
  },
  图像: {
    skill: "image-edit",
    calls: [
      "图像插件 · 读取原图与选区",
      "图像插件 · 按目标调整图层",
      "图像插件 · 导出图像",
    ],
    output: "编辑源文件与导出图像",
    preview: "图像插件 · 生成原尺寸预览与局部对比图",
  },
  脚本: {
    skill: "script-edit",
    calls: [
      "Codex · 读取脚本与调用点",
      "Codex 插件 · 修改相关代码",
      "Godot MCP · 检查脚本解析与目标行为",
    ],
    output: "脚本文件与变更说明",
    preview: "Godot MCP · 运行目标场景，记录行为与画面",
  },
  音频: {
    skill: "audio-edit",
    calls: [
      "音频插件 · 读取音轨与时间选区",
      "音频插件 · 调整目标片段",
      "音频插件 · 导出音频",
    ],
    output: "音频工程与导出音频",
    preview: "音频插件 · 生成试听片段与波形对比",
  },
  翻译: {
    skill: "translation-edit",
    calls: [
      "Codex · 读取源文本与术语表",
      "Codex 插件 · 修改目标条目",
      "本地化插件 · 导出语言资源",
    ],
    output: "翻译资源与术语变更记录",
    preview: "Godot MCP · 预览目标语言的界面排版",
  },
  其他: {
    skill: "resource-edit",
    calls: [
      "Codex · 识别资源格式与依赖",
      "资源插件 · 按目标编辑内容",
      "资源插件 · 导出资源",
    ],
    output: "修改后的资源与依赖清单",
    preview: "资源插件 · 生成可检查的预览与差异摘要",
  },
};

// UI examples only: these labels do not resolve or invoke skills and plugins.
export function mockWorkflow(
  task: DemoTask,
  object: DemoObject,
): WorkflowStep[] {
  const recipe = recipes[object.objectType];
  const goal = task.prompt ?? task.detail;
  const context = `${object.id} / ${object.name} / ${object.version}`;
  const references =
    task.annotations?.map((item) => `序号 ${item.number}`) ?? [];
  return [
    {
      title: "解析目标与标注",
      skill: "goal-to-workflow",
      calls: [
        "Beaver · 读取任务目标与对象上下文",
        "Codex · 整理修改范围、约束与交付要求",
      ],
      instruction: `将“${task.title}”整理为可执行的制作规格；对应原始要求：${goal}`,
      inputs: [
        context,
        goal,
        references.length
          ? `标注：${references.join("、")}`
          : "操作范围：整个对象",
      ],
      outputs: ["修改目标、标注映射、交付清单与验收规格"],
      acceptance: "每条人类要求都有对应操作；不确定的关键条件进入追问。",
    },
    {
      title: "准备版本与工具",
      skill: "prepare-object-workspace",
      calls: [
        "Beaver · 等待对象写入队列并建立工作快照",
        "Codex · 检查所需工具、插件及版本",
      ],
      instruction: `从 ${context} 准备本次任务的独立工作版本，并检查 ${recipe.skill} 所需的环境。`,
      inputs: ["上一步的制作规格", context],
      outputs: ["工作快照、对象写入占用记录、工具与插件清单"],
      acceptance: "原版本可回溯；同一对象仅允许当前任务写入；工具环境可用。",
    },
    {
      title: task.summary ?? task.title,
      skill: recipe.skill,
      calls: recipe.calls,
      instruction: `在工作版本中完成“${task.title}”。${goal}${references.length ? ` 按 ${references.join("、")} 的上下文定位修改区域。` : ""}`,
      inputs: [
        "工作快照与制作规格",
        ...object.components.map((item) => `${item.name} · ${item.format}`),
      ],
      outputs: [recipe.output, "本步骤的修改记录"],
      acceptance: "结果覆盖目标要求，关联资源完整，修改范围与规格一致。",
    },
    {
      title: "生成预览与局部检查",
      skill: "preview-object-change",
      calls: [recipe.preview, "Codex · 对照本次任务的验收规格"],
      instruction:
        "为修改位置生成前后对照，检查本次涉及的内容，并保留供用户查看的预览。",
      inputs: [recipe.output, "修改前快照与本任务的验收规格"],
      outputs: ["对象预览、标注区域对照、局部检查记录"],
      acceptance: "修改位置可辨认；相关资源加载正确；本次修改满足对应规格。",
    },
    {
      title: "提交交付与人工验收",
      skill: "deliver-object-iteration",
      calls: [
        "Beaver · 登记版本、子对象与制作记录",
        "Beaver · 提交交付供用户验收",
      ],
      instruction:
        "把本任务的交付内容关联回对象与迭代栈。确认后再推进依赖本结果的任务。",
      inputs: ["制作结果、预览与局部检查记录"],
      outputs: ["候选对象版本、交付清单、可回溯的流程记录"],
      acceptance: "用户可以查看结果与来源，批准交付或提出下一次迭代。",
    },
  ];
}

export function workflowStepStatus(
  task: DemoTask,
  index: number,
  count: number,
) {
  const position = (Math.min(100, Math.max(0, task.progress)) / 100) * count;
  if (index < Math.floor(position)) {
    return index === count - 1 && task.status !== "已验收"
      ? "待验收"
      : "已完成";
  }
  if (index > Math.floor(position)) return "等待前序";
  if (task.status === "执行失败") return "执行失败";
  if (task.status === "排队中" || task.status === "等待依赖")
    return task.status;
  return "执行中";
}
