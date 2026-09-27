import assert from "node:assert/strict";
import test from "node:test";
import {
  parseObjectCommand,
  parseObjectCommandResult,
  type ObjectCommand,
} from "../src/shared/object-command";
import type {
  ObjectCatalogRecord,
  ObjectCatalogVersion,
} from "../src/shared/object-catalog";

const file = {
  path: "hero.txt",
  role: "source",
  bytes: 4,
  sha256: "a".repeat(64),
};

function manifest(
  status: "captured" | "accepted",
  versionId = "v1",
): Record<string, unknown> {
  return {
    schemaVersion: 1,
    projectId: "project",
    objectId: "object",
    versionId,
    status,
    name: "Hero",
    category: "图像",
    tags: ["hero"],
    thumbnailPath: "thumbs/hero.png",
    parentObjectId: null,
    components: [],
    files: [file],
    references: [],
  };
}

function version(
  status: "captured" | "accepted",
  versionId = "v1",
): ObjectCatalogVersion {
  return { versionId, manifest: manifest(status, versionId) };
}

function record(
  overrides: Partial<ObjectCatalogRecord> = {},
): ObjectCatalogRecord {
  return {
    id: "object",
    projectId: "project",
    name: "Hero",
    category: "图像",
    tags: ["hero"],
    thumbnailPath: "thumbs/hero.png",
    parentObjectId: null,
    revision: 1,
    components: [],
    files: [{ path: file.path, role: file.role }],
    references: [],
    versions: [],
    ...overrides,
  };
}

function acceptanceCommand(): ObjectCommand {
  return parseObjectCommand({
    method: "object.acceptVersion",
    input: {
      projectId: "project",
      requestId: "accept-request",
      objectId: "object",
      versionId: "v1",
      expectedRevision: 1,
    },
  });
}

function acceptanceFixture() {
  const original = record({
    versions: [version("captured"), version("captured", "v2")],
  });
  const accepted = record({
    revision: original.revision + 1,
    versions: [version("accepted"), version("captured", "v2")],
  });
  const command = acceptanceCommand();
  const result = {
    projectId: "project",
    requestId: "accept-request",
    object: accepted,
    versionId: "v1",
  };
  return { original, accepted, command, result };
}

test("acceptance command schema requires a pinned version and rejects extras", () => {
  const command = acceptanceCommand();
  assert.equal(command.method, "object.acceptVersion");
  assert.equal(command.input.versionId, "v1");
  assert.throws(() =>
    parseObjectCommand({
      method: "object.acceptVersion",
      input: {
        projectId: "project",
        requestId: "accept-request",
        objectId: "object",
        expectedRevision: 1,
      },
    }),
  );
  assert.throws(() =>
    parseObjectCommand({
      method: "object.acceptVersion",
      input: {
        projectId: "project",
        requestId: "accept-request",
        objectId: "object",
        versionId: "v1",
        expectedRevision: 1,
        force: true,
      },
    }),
  );
  assert.throws(() =>
    parseObjectCommand({
      method: "object.acceptVersion",
      input: {
        projectId: "project",
        requestId: "accept-request",
        objectId: "object",
        versionId: "v1",
        expectedRevision: 0.5,
      },
    }),
  );
});

test("acceptance receipt permits only captured to accepted status change", () => {
  const { original, command, result } = acceptanceFixture();
  assert.deepEqual(parseObjectCommandResult(result, command, original), result);

  const capture = parseObjectCommand({
    method: "object.captureVersion",
    input: {
      projectId: "project",
      requestId: "capture-request",
      objectId: "object",
      expectedRevision: 1,
    },
  });
  assert.throws(() => parseObjectCommandResult(result, capture, original));
});

test("acceptance receipt rejects identity, revision, content and history tampering", () => {
  const { original, command, result } = acceptanceFixture();
  const invalid = (mutate: (copy: typeof result) => void) => {
    const copy = structuredClone(result);
    mutate(copy);
    assert.throws(() => parseObjectCommandResult(copy, command, original));
  };

  invalid((copy) => {
    copy.versionId = "v2";
  });
  invalid((copy) => {
    copy.object.revision = original.revision;
  });
  invalid((copy) => {
    const manifest = copy.object.versions[0]!.manifest as Record<
      string,
      unknown
    >;
    (manifest.files as Array<Record<string, unknown>>)[0]!.sha256 = "b".repeat(
      64,
    );
  });
  invalid((copy) => {
    const manifest = copy.object.versions[0]!.manifest as Record<
      string,
      unknown
    >;
    manifest.components = [{ id: "tampered", kind: "scene", name: "Tampered" }];
  });
  invalid((copy) => {
    copy.object.versions[1]!.manifest = manifest("accepted", "v2");
  });
  invalid((copy) => {
    copy.object.files[0]!.path = "different.txt";
  });
  invalid((copy) => {
    copy.object.versions.pop();
  });
});
