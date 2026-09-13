import fs from "node:fs/promises";
import path from "node:path";
import { Preferences } from "./settings";
import type { Task } from "../shared/types";
import { findTool } from "./process";

export function codeStructureInstructions(resources: string): Promise<string> {
  return fs.readFile(
    path.join(resources, "instructions", "code-structure.md"),
    "utf8",
  );
}

export async function prepareHome(
  root: string,
  resources: string,
  mcpScript: string,
  task: Pick<Task, "id" | "workspace" | "capability">,
  prefs: Preferences,
): Promise<NodeJS.ProcessEnv> {
  const home = path.join(root, "codex", task.id);
  await fs.mkdir(home, { recursive: true });
  await fs.cp(path.join(resources, "skills"), path.join(home, "skills"), {
    recursive: true,
  });
  const provider = prefs.resolve(task.capability);
  const env: NodeJS.ProcessEnv = { ...process.env };
  for (const key of Object.keys(env))
    if (/^(CODEX_|OPENAI_)/.test(key)) delete env[key];
  env.CODEX_HOME = home;
  env.BEAVER_CODEX_KEY = provider.key;
  env.BEAVER_PROJECT_ROOT = task.workspace;
  env.BEAVER_MEDIA_PROVIDERS = JSON.stringify(
    Object.fromEntries(
      ["image", "speech", "music", "translation"].map((c) => {
        try {
          return [
            c,
            prefs.resolve(c as "image" | "speech" | "music" | "translation"),
          ];
        } catch {
          return [c, null];
        }
      }),
    ),
  );
  const q = JSON.stringify;
  const settings = prefs.read();
  const tools = { ...settings.tools };
  for (const name of ["godot", "blender", "node"] as const) {
    try {
      tools[name] = await findTool(name, tools[name]);
    } catch {
      tools[name] = "not installed";
    }
  }
  if (tools.node !== "not installed")
    env.PATH = path.dirname(tools.node) + path.delimiter + (env.PATH ?? "");
  let config = `model = ${q(provider.model)}\nmodel_provider = "beaver"\napproval_policy = "never"\nsandbox_mode = "danger-full-access"\n[model_providers.beaver]\nname = "Beaver configured AI"\nbase_url = ${q(provider.baseUrl)}\nwire_api = "responses"\n${provider.key ? 'env_key = "BEAVER_CODEX_KEY"\n' : ""}[mcp_servers.beaver_media]\ncommand = ${q(process.execPath)}\nargs = [${q(mcpScript)}]\nenv_vars = ["BEAVER_PROJECT_ROOT", "BEAVER_MEDIA_PROVIDERS"]\n[mcp_servers.beaver_media.env]\nELECTRON_RUN_AS_NODE = "1"\n`;
  if (settings.mcp.godot)
    config +=
      '[mcp_servers.godot]\ncommand = "npx"\nargs = ["--yes", "@coding-solo/godot-mcp@0.1.1"]\n' +
      (tools.godot !== "not installed"
        ? `[mcp_servers.godot.env]\nGODOT_PATH = ${q(tools.godot)}\n`
        : "");
  if (settings.mcp.blender)
    config +=
      '[mcp_servers.blender]\ncommand = "uvx"\nargs = ["blender-mcp==1.9.1"]\n[mcp_servers.blender.env]\nDISABLE_TELEMETRY = "true"\n';
  await fs.writeFile(path.join(home, "config.toml"), config, "utf8");
  const codeStructure = await codeStructureInstructions(resources);
  await fs.writeFile(
    path.join(home, "AGENTS.md"),
    `# Beaver execution environment\nWork only in the supplied project copy. Do not edit the original project or application state. Complete the user's game goal autonomously and verify the result. Do not claim unavailable tools or failed exports succeeded. Use the supplied game-production skills. Detected executable paths: ${JSON.stringify(tools)}.\n\n${codeStructure}`,
    "utf8",
  );
  return env;
}
