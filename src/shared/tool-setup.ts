import type { ToolStatus } from "./types";
export type ToolName = ToolStatus["name"];
export function matchesToolVersion(name: ToolName, version: string): boolean {
  const patterns: Record<ToolName, RegExp> = {
    node: /^v\d+\.\d+\.\d+(?:\s|$)/,
    codex: /^codex(?:-cli)?\s+\d+\.\d+/i,
    godot: /^(?:Godot Engine v)?4\.\d+(?:\.|\s|$)/i,
    blender: /^Blender\s+\d+\.\d+/i,
  };
  return patterns[name].test(version.trim());
}
export interface ToolSetupState {
  status: "idle" | "running" | "completed" | "failed" | "cancelled";
  active?: ToolName;
  error?: string;
  updatedAt: string;
  steps: {
    name: ToolName;
    status:
      "waiting" | "checking" | "installing" | "ready" | "failed" | "cancelled";
    result?: ToolStatus;
  }[];
}
