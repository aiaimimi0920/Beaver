import type { CSSProperties } from "react";
import type { IconName } from "../Icon";
export type ObjectType =
  "图像" | "音频" | "模型" | "场景" | "翻译" | "脚本" | "其他";

export const objectCategories: {
  name: ObjectType | "全部";
  icon: IconName;
  color: string;
}[] = [
  { name: "全部", icon: "overview", color: "#f0f2f4" },
  { name: "图像", icon: "assets", color: "#ff6647" },
  { name: "音频", icon: "music", color: "#b77bff" },
  { name: "模型", icon: "features", color: "#3694ff" },
  { name: "场景", icon: "city", color: "#e6b83d" },
  { name: "翻译", icon: "translation", color: "#80bd39" },
  { name: "脚本", icon: "code", color: "#39d5ad" },
  { name: "其他", icon: "folder", color: "#b8e7ef" },
];

export function objectCardStyle(type: ObjectType): CSSProperties {
  return {
    "--object-type-color": objectCategories.find((item) => item.name === type)
      ?.color,
  } as CSSProperties;
}
