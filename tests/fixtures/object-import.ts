import type { ImportPreparationRequest } from "../../src/shared/object-import";

export function inspection(projectId = "source-1") {
  const objects = ["hero", "prop"].map((objectId) => ({
    id: objectId,
    projectId,
    name: `Working ${objectId}`,
    components: [],
    files: [{ path: "live-only.tscn", role: "scene" }],
    references: [{ projectId, objectId: "prop", versionId: null }],
    versions: [1, 2, 3].map((version) => ({
      versionId: `${objectId}-v${version}`,
      manifest: {
        schemaVersion: 1,
        projectId,
        objectId,
        versionId: `${objectId}-v${version}`,
        status: version === 3 ? "candidate" : "accepted",
        name: `Frozen ${objectId} ${version}`,
        components: [],
        references: [],
        files: [
          {
            path: `${objectId}-${version}.tscn`,
            role: "scene",
            bytes: 8,
            sha256: "a".repeat(64),
          },
        ],
      },
    })),
  }));
  return {
    project: { id: projectId },
    objects,
    importVersions: objects.flatMap((object) =>
      object.versions.map((version, index) => ({
        objectId: object.id,
        versionId: version.versionId,
        sourceDigest: index === 2 ? null : (index === 0 ? "a" : "b").repeat(64),
        blocker: index === 2 ? "IMPORT_VERSION_NOT_ACCEPTED" : null,
      })),
    ),
  };
}

export function receipt(request: ImportPreparationRequest) {
  return {
    schemaVersion: 1,
    preparationId: `import-${request.requestId}`,
    requestId: request.requestId,
    targetProjectId: request.targetProjectId,
    sourceProjectId: request.source.projectId,
    sourceObjectId: request.objectId,
    baseline: request.baseline,
    acceptedVersionId: request.baseline.versionId,
    sourceDigest: request.sourceDigest,
    readyToCommit: false,
  };
}

export function deferred() {
  let resolve!: (value: unknown) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<unknown>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
}
