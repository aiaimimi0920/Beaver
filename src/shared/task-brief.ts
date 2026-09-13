import {
  audiences,
  campaignChoices,
  gameSizes,
  genreChoices,
  phases,
  themeChoices,
} from "./blueprint-catalog";
import { styles, formatGameBrief } from "./game-design";
import type { ProjectBlueprint } from "./project-blueprint";
import type { Task } from "./types";

export interface TaskProjectContext {
  name: string;
  revision: number;
  blueprint: ProjectBlueprint;
}

export function formatTaskBrief(
  task: Pick<Task, "design" | "projectContext">,
): string {
  const context = task.projectContext;
  if (!context) return formatGameBrief(task.design);
  const b = context.blueprint;
  const audience = audiences.find((item) => item.id === b.audience)!;
  const size = gameSizes.find((item) => item.id === b.size)!;
  return [
    `项目创作约定：${context.name}（任务创建时冻结，规划版本 ${context.revision}）`,
    `主类别：${genreChoices.find((item) => item.id === b.genres[0])?.name ?? b.genres[0]}`,
    `副类别：${genreChoices.find((item) => item.id === b.genres[1])?.name ?? b.genres[1] ?? "无"}`,
    `主题：${b.theme.mode === "custom" ? b.theme.value : (themeChoices.find((item) => item.id === b.theme.value)?.name ?? b.theme.value)}`,
    `目标评级：${audience.name}。${audience.meaning} 这是创作尺度，不是已取得官方评级。`,
    `游戏大小：${size.name}。${size.meaning}`,
    `表现风格：${styles.find((item) => item.id === b.style)?.name ?? b.style}`,
    b.online.enabled
      ? `在线多人：开启；拓扑：${b.online.topology === "dedicated" ? "专用服务器" : "房主与中继"}；每房间 ${b.online.playersPerSession} 人；目标峰值 ${b.online.peakCcu} 人；初期实例 ${b.online.serverCount}；区域：${b.online.region}。这些数字是设计目标，不是已部署或已压测结果。`
      : "在线多人：关闭。不自行添加在线服务器、账号或联机依赖。",
    "阶段投入优先级（0-5，不是完成度，不自动把所有项目变成本次任务）：",
    ...phases.map(
      (phase) =>
        `${phase.name}：${phase.metrics.map((metric) => `${metric.name} ${b.priorities[metric.id]}`).join("、")}`,
    ),
    `规划功能包 ID：${b.plannedFeatures.join("、") || "无"}。勾选仅表示意向，不代表已安装、已实现或授权一次性接入全部功能。`,
    ...campaignChoices
      .filter(
        (choice) =>
          b.campaigns[choice.id].notes ||
          b.campaigns[choice.id].channels.length,
      )
      .map(
        (choice) =>
          `${choice.name}草案：${b.campaigns[choice.id].channels.join("、")}；${b.campaigns[choice.id].notes}`,
      ),
    "发行与宣传信息仅供准备内容，不自动发布、联系媒体或付费投放。",
    "围绕本次明确目标自行寻找上下文。若目标与已锁定的类别、评级、规模或联机设定矛盾，调用 beaver_ask_user 询问，不静默改变基本设定。",
    "保持原创或使用有权使用的内容；按实际运行结果报告，不把规划数值或功能意向当作已完成成果。",
  ].join("\n");
}
