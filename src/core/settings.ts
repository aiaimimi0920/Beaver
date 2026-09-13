import { z } from "zod";
import {
  capabilities,
  defaultSettings,
  type Capability,
  type Settings,
} from "../shared/types";
import { Store } from "./store";
import type { LocalApiDefaults } from "./local-codex";

const endpoint = z
  .object({
    baseUrl: z.string().max(2000),
    model: z.string().max(200),
    route: z.string().max(200),
    hasKey: z.boolean().optional(),
  })
  .strict();
const endpointSet = z.object({
  code: endpoint,
  review: endpoint,
  image: endpoint,
  speech: endpoint,
  music: endpoint,
  translation: endpoint,
});
export const settingsSchema = z
  .object({
    mode: z.enum(["local", "cloud"]),
    local: endpointSet,
    cloud: endpoint,
    cloudModels: z.object({
      code: z.string(),
      review: z.string(),
      image: z.string(),
      speech: z.string(),
      music: z.string(),
      translation: z.string(),
    }),
    tools: z.object({
      codex: z.string(),
      godot: z.string(),
      blender: z.string(),
      node: z.string(),
    }),
    mcp: z.object({ godot: z.boolean(), blender: z.boolean() }),
    maxParallel: z.number().int().min(1).max(6),
    askRatio: z
      .union([
        z.literal(0),
        z.literal(10),
        z.literal(30),
        z.literal(70),
        z.literal(100),
      ])
      .default(100),
  })
  .strict();
export interface Vault {
  encrypt(text: string): string;
  decrypt(cipher: string): string;
}
export function validBase(value: string): string {
  const url = new URL(value);
  if (
    !["https:", "http:"].includes(url.protocol) ||
    url.username ||
    url.password ||
    url.search ||
    url.hash
  )
    throw new Error("API 地址必须是无凭据、无查询参数的 HTTP(S) URL");
  if (
    url.protocol === "http:" &&
    !["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
  )
    throw new Error("非本机 API 必须使用 HTTPS，避免密钥明文传输");
  return value.replace(/\/+$/, "");
}
export class Preferences {
  constructor(
    private store: Store,
    private vault: Vault,
  ) {}
  read(): Settings {
    const settings =
      this.store.get<Settings>("settings", "main") ?? defaultSettings();
    for (const c of capabilities)
      settings.local[c].hasKey = !!this.store.get("secret", c);
    settings.cloud.hasKey = !!this.store.get("secret", "cloud");
    return settings;
  }
  save(value: unknown, keys: Record<string, string>): Settings {
    const settings = settingsSchema.parse(value);
    for (const ep of [...Object.values(settings.local), settings.cloud])
      if (ep.baseUrl) validBase(ep.baseUrl);
    const encrypted: [string, string][] = [];
    for (const [slot, key] of Object.entries(keys)) {
      if (![...capabilities, "cloud"].includes(slot))
        throw new Error("未知凭据类型");
      if (key) encrypted.push([slot, this.vault.encrypt(key)]);
    }
    this.store.transaction(() => {
      for (const [slot, cipher] of encrypted)
        this.store.put("secret", slot, cipher);
      this.store.put("settings", "main", settings);
    });
    return this.read();
  }
  clearKey(slot: string): void {
    if (![...capabilities, "cloud"].includes(slot))
      throw new Error("未知凭据类型");
    this.store.put("secret", slot, "");
  }
  importLocalDefaults(
    source: LocalApiDefaults,
    value: unknown = this.read(),
    keys: Record<string, string> = {},
  ): { settings: Settings; filled: Capability[] } {
    const settings = settingsSchema.parse(value);
    const baseUrl = validBase(source.baseUrl);
    const nextKeys = { ...keys };
    const filled: Capability[] = [];
    for (const cap of ["code", "review", "translation"] as const) {
      const ep = settings.local[cap];
      const hasKey = !!nextKeys[cap] || !!this.store.get("secret", cap);
      if (ep.baseUrl && validBase(ep.baseUrl) !== baseUrl) continue;
      if (!ep.baseUrl && (ep.model || hasKey)) continue;
      if (!ep.baseUrl || !ep.model || !hasKey) {
        ep.baseUrl ||= baseUrl;
        ep.model ||= source.model;
        if (!hasKey) nextKeys[cap] = source.key;
        filled.push(cap);
      }
    }
    return { settings: this.save(settings, nextKeys), filled };
  }
  key(slot: string): string {
    const cipher = this.store.get<string>("secret", slot);
    return cipher ? this.vault.decrypt(cipher) : "";
  }
  resolve(capability: Capability): {
    baseUrl: string;
    model: string;
    route: string;
    key: string;
  } {
    const settings = this.read();
    const cloud = settings.mode === "cloud";
    const ep = cloud ? settings.cloud : settings.local[capability];
    const model = cloud ? settings.cloudModels[capability] : ep.model;
    if (!ep.baseUrl || !model)
      throw new Error(
        `请先配置${cloud ? "平台" : "自定义"} ${capability} 服务地址与模型`,
      );
    return {
      baseUrl: validBase(ep.baseUrl),
      model,
      route: cloud ? settings.local[capability].route : ep.route,
      key: this.key(cloud ? "cloud" : capability),
    };
  }
  redact(text: string): string {
    for (const slot of [...capabilities, "cloud"]) {
      const key = this.key(slot);
      if (key) text = text.split(key).join("[REDACTED_SECRET]");
    }
    return text.replace(
      /Bearer\s+[A-Za-z0-9._~+\/-]+/gi,
      "Bearer [REDACTED_SECRET]",
    );
  }
}
