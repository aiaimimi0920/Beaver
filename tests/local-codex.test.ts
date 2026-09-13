import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { readLocalCodex } from "../src/core/local-codex";
import { Store } from "../src/core/store";
import { Preferences } from "../src/core/settings";
import { defaultSettings } from "../src/shared/types";

test("explicit local Codex import reads API auth without copying personal configuration", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-api-"));
  try {
    await fs.writeFile(
      path.join(root, "config.toml"),
      'model="fixture-model"\nmodel_provider="local"\n[model_providers.local]\nbase_url="http://127.0.0.1:8317/v1"\nwire_api="responses"\n',
    );
    await fs.writeFile(
      path.join(root, "auth.json"),
      JSON.stringify({ OPENAI_API_KEY: "fixture-secret" }),
    );
    const provider = await readLocalCodex({ CODEX_HOME: root });
    assert.deepEqual(provider, {
      baseUrl: "http://127.0.0.1:8317/v1",
      model: "fixture-model",
      key: "fixture-secret",
    });
    assert.deepEqual(Object.keys(provider).sort(), ["baseUrl", "key", "model"]);
    await fs.writeFile(
      path.join(root, "auth.json"),
      JSON.stringify({ tokens: { access_token: "oauth-must-not-copy" } }),
    );
    await assert.rejects(
      readLocalCodex({ CODEX_HOME: root }),
      /未找到可导入的 API Key/,
    );
    await fs.appendFile(
      path.join(root, "config.toml"),
      'experimental_bearer_token="explicit-provider-api-key"\n',
    );
    assert.equal(
      (await readLocalCodex({ CODEX_HOME: root })).key,
      "explicit-provider-api-key",
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("local Codex profiles and declared environment credentials are respected", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-api-"));
  try {
    await fs.writeFile(
      path.join(root, "config.toml"),
      'model="base"\nmodel_provider="local"\n[profiles.game]\nmodel="game-model"\n[model_providers.local]\nbase_url="http://localhost:8317/v1"\nenv_key="TEST_BEAVER_KEY"\n',
    );
    const env = {
      CODEX_HOME: root,
      CODEX_PROFILE: "game",
      TEST_BEAVER_KEY: "environment-secret",
    };
    assert.equal((await readLocalCodex(env)).model, "game-model");
    assert.equal((await readLocalCodex(env)).key, "environment-secret");
    await assert.rejects(
      readLocalCodex({ ...env, TEST_BEAVER_KEY: "" }),
      /环境变量不可用/,
    );
    await fs.writeFile(
      path.join(root, "config.toml"),
      'secret="must-not-leak\n',
    );
    await assert.rejects(
      readLocalCodex(env),
      (error: unknown) =>
        error instanceof Error && !error.message.includes("must-not-leak"),
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("local defaults fill only compatible blank text slots and never expose keys", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-defaults-"));
  const store = new Store(root);
  const prefs = new Preferences(store, {
    encrypt: (value) => Buffer.from(value).toString("base64"),
    decrypt: (value) => Buffer.from(value, "base64").toString(),
  });
  try {
    const before = defaultSettings();
    before.local.review = {
      baseUrl: "https://review.example/v1",
      model: "independent",
      route: "",
    };
    prefs.save(before, { review: "keep-review-secret" });
    const source = {
      baseUrl: "http://127.0.0.1:8317/v1",
      model: "fixture",
      key: "source-secret",
    };
    const result = prefs.importLocalDefaults(source);
    assert.deepEqual(result.filled, ["code", "translation"]);
    assert.equal(prefs.resolve("code").key, source.key);
    assert.equal(prefs.resolve("review").key, "keep-review-secret");
    assert.equal(result.settings.local.image.baseUrl, "");
    assert.equal(result.settings.local.image.hasKey, false);
    assert.ok(!JSON.stringify(result).includes(source.key));
    assert.notEqual(store.get("secret", "code"), source.key);
    assert.deepEqual(
      prefs.importLocalDefaults({ ...source, key: "new-secret" }).filled,
      [],
    );
    assert.equal(prefs.key("code"), source.key);
    const draft = prefs.read();
    draft.maxParallel = 4;
    prefs.importLocalDefaults(source, draft, { code: "user-draft-key" });
    assert.equal(prefs.key("code"), "user-draft-key");
    assert.equal(prefs.read().maxParallel, 4);
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
