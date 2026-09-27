import type { ImportHistoryEntry } from "../../src/shared/object-import-history";
export const hash = "a".repeat(64);
export function historyEntry(
  kind: "project" | "files",
  suffix = "one",
): ImportHistoryEntry {
  const preparationId = "receipt-" + suffix;
  return {
    kind,
    preparationId,
    requestId: "request-" + suffix,
    cursor:
      (kind === "files"
        ? "file_object_import_preparation:"
        : "object_import_preparation:") + preparationId,
  };
}
export function projectReceipt(entry = historyEntry("project")) {
  return {
    schemaVersion: 1,
    preparationId: entry.preparationId,
    requestId: entry.requestId,
    requestDigest: hash,
    targetProjectId: "target",
    sourcePath: "Z:/offline/project",
    sourceProjectId: "source",
    sourceObjectId: "hero",
    acceptedVersionId: "v1",
    sourceDigest: hash,
    baseline: { kind: "latestAccepted" },
    readyToCommit: false,
    versions: [
      {
        schemaVersion: 1,
        projectId: "source",
        objectId: "hero",
        versionId: "v1",
        status: "accepted",
        name: "Frozen hero",
        components: [{ id: "mesh", kind: "mesh", name: "Body" }],
        files: [{ path: "hero.tscn", role: "scene", bytes: 12, sha256: hash }],
        references: [] as {
          projectId: string;
          objectId: string;
          versionId: string;
        }[],
      },
    ],
    identityMap: {
      objects: { hero: "target-hero" },
      components: { mesh: "target-mesh" },
      versions: { v1: "target-v1" },
    },
  };
}
export function filesReceipt(entry = historyEntry("files")) {
  return {
    schemaVersion: 1,
    preparationId: entry.preparationId,
    requestId: entry.requestId,
    requestDigest: hash,
    targetProjectId: "target",
    readyToCommit: false,
    source: {
      source: { kind: "files", paths: ["Z:/offline/image.png"] },
      digest: hash,
      files: [
        {
          sourcePath: "Z:/offline/image.png",
          path: "Z:/offline/image.png",
          relativePath: "image.png",
          kind: "image",
          bytes: 5,
          sha256: hash,
        },
      ],
    },
    groups: [
      { id: "group", name: "Saved group", paths: ["Z:/offline/image.png"] },
    ],
    identityMap: {
      files: { "Z:/offline/image.png": "target-file" },
      groups: { group: "target-group" },
    },
  };
}
export const page = (
  entries: ImportHistoryEntry[],
  next: string | null = null,
) => ({ targetProjectId: "target", entries, next });
