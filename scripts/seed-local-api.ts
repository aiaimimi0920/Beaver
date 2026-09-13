import { app, safeStorage } from "electron";
import fs from "node:fs/promises";
import path from "node:path";
import { Store } from "../src/core/store";
import { Preferences } from "../src/core/settings";
import { readLocalCodex } from "../src/core/local-codex";

// Explicit maintenance command; never runs on normal application startup.
void app.whenReady().then(async () => {
  let store: Store | undefined;
  try {
    const argument = process.argv.indexOf("--data-dir");
    const directory = argument >= 0 ? process.argv[argument + 1] : undefined;
    if (!directory)
      throw new Error("An explicit existing Beaver data directory is required");
    const root = await fs.realpath(directory);
    await fs.access(path.join(root, "beaver.sqlite"));
    if (
      !safeStorage.isEncryptionAvailable() ||
      (process.platform === "linux" &&
        safeStorage.getSelectedStorageBackend() === "basic_text")
    )
      throw new Error("Secure credential storage is unavailable");
    const source = await readLocalCodex();
    store = new Store(root);
    const prefs = new Preferences(store, {
      encrypt: (text) => safeStorage.encryptString(text).toString("base64"),
      decrypt: (cipher) =>
        safeStorage.decryptString(Buffer.from(cipher, "base64")),
    });
    const result = prefs.importLocalDefaults(source);
    const verified = result.filled.every(
      (cap) => prefs.key(cap) === source.key,
    );
    if (!verified) throw new Error("Encrypted credential verification failed");
    console.log(
      JSON.stringify({
        success: true,
        dataDirectory: root,
        filled: result.filled,
        verified,
        key: "[REDACTED_SECRET]",
      }),
    );
  } catch {
    console.error("Local API setup failed; no credential details logged");
    process.exitCode = 1;
  } finally {
    store?.close();
    app.exit(Number(process.exitCode) || 0);
  }
});
