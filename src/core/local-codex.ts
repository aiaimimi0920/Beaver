import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parse } from "smol-toml";
import { validBase } from "./settings";

export interface LocalApiDefaults {
  baseUrl: string;
  model: string;
  key: string;
}

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

export async function readLocalCodex(
  env: NodeJS.ProcessEnv = process.env,
): Promise<LocalApiDefaults> {
  const home = env.CODEX_HOME || path.join(os.homedir(), ".codex");
  let config: Record<string, unknown>;
  try {
    config = parse(await fs.readFile(path.join(home, "config.toml"), "utf8"));
  } catch {
    // Parser errors can include source lines containing credentials.
    throw new Error("无法读取本机 Codex 配置，请手动填写 API 设置");
  }
  const profileName = env.CODEX_PROFILE || config.profile;
  if (profileName) {
    const profile = record(record(config.profiles)[String(profileName)]);
    if (!Object.keys(profile).length)
      throw new Error("本机 Codex profile 不存在");
    config = { ...config, ...profile };
  }
  const id =
    typeof config.model_provider === "string"
      ? config.model_provider
      : "openai";
  const provider = record(record(config.model_providers)[id]);
  if (provider.wire_api && provider.wire_api !== "responses")
    throw new Error("本机 Codex 服务不是 Responses API，请手动配置兼容服务");
  const baseUrl = validBase(
    typeof provider.base_url === "string"
      ? provider.base_url
      : id === "openai"
        ? env.OPENAI_BASE_URL || "https://api.openai.com/v1"
        : "",
  );
  if (typeof config.model !== "string" || !config.model.trim())
    throw new Error("本机 Codex 未指定模型，请手动选择模型");
  let key = "";
  if (typeof provider.env_key === "string") {
    key = env[provider.env_key] || "";
    if (!key) throw new Error("本机 Codex 指定的 API Key 环境变量不可用");
  } else if (typeof provider.experimental_bearer_token === "string") {
    key = provider.experimental_bearer_token;
  } else {
    key = env.OPENAI_API_KEY || "";
    if (!key) {
      try {
        const auth = record(
          JSON.parse(await fs.readFile(path.join(home, "auth.json"), "utf8")),
        );
        if (typeof auth.OPENAI_API_KEY === "string") key = auth.OPENAI_API_KEY;
      } catch {
        // Never return auth file contents or execute commands from its config.
      }
    }
  }
  if (!key.trim())
    throw new Error(
      "未找到可导入的 API Key；不导入 ChatGPT 登录令牌或执行凭据命令",
    );
  return { baseUrl, model: config.model, key };
}
