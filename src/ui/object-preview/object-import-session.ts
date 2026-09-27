import {
  parseImportInspection,
  parseImportReceipt,
  parseFileImportReceipt,
  parseFileSourceSnapshot,
  type FileSourceSnapshot,
  type ImportPreparationReceipt,
  type ImportPreparationRequest,
  type FileImportPreparationRequest,
  type FileImportPreparationReceipt,
  type ImportSourceObject,
} from "../../shared/object-import";
import type { ImportDraft } from "./object-import-draft";
import { addImportGroup, assignImportGroup } from "./object-import-groups";

export type ObjectImportSourceMode = "project" | "external" | "files";

export interface FileGroup {
  id: string;
  name: string;
  paths: string[];
}

export interface ObjectImportState {
  sourceMode: ObjectImportSourceMode;
  path: string;
  sourceProjectId: string;
  selectedPaths: string[];
  fileSnapshot: FileSourceSnapshot | null;
  fileGroups: FileGroup[];
  objects: ImportSourceObject[];
  objectId: string;
  versionId: string;
  phase: "editing" | "inspecting" | "ready" | "preparing" | "prepared";
  receipt: ImportPreparationReceipt | FileImportPreparationReceipt | null;
  error: string;
}

type ImportApi = (method: string, input: unknown) => Promise<unknown>;
const message = (error: unknown) =>
  error instanceof Error ? error.message : String(error);
const latest = (object?: ImportSourceObject) =>
  object?.versions.findLast((version) => version.sourceDigest)?.versionId ??
  object?.versions[0]?.versionId ??
  "";
const normalizeFilePath = (path: string) => {
  const normalized = path.trim().replaceAll("\\", "/");
  if (!normalized) return "";
  if (normalized === "/" || /^[A-Za-z]:\/$/.test(normalized)) return normalized;
  return normalized.replace(/\/+$/, "");
};

export class ObjectImportSession {
  private state: ObjectImportState = {
    sourceMode: "project",
    path: "",
    sourceProjectId: "",
    selectedPaths: [],
    fileSnapshot: null,
    fileGroups: [],
    objects: [],
    objectId: "",
    versionId: "",
    phase: "editing",
    receipt: null,
    error: "",
  };
  private revision = 0;
  private pickerSequence = 0;
  private request?: ImportPreparationRequest;
  private fileRequest?: FileImportPreparationRequest;
  private listeners = new Set<() => void>();

  constructor(
    readonly targetProjectId: string | undefined,
    private readonly call: ImportApi,
    private readonly requestId: () => string = () => crypto.randomUUID(),
    draft?: ImportDraft,
  ) {
    if (draft) {
      const { groupName: _groupName, ...selection } = draft;
      this.state = { ...this.state, ...structuredClone(selection) };
    }
  }

  getSnapshot = (): Readonly<ObjectImportState> => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  get selectedObject() {
    return this.state.objects.find(
      (object) => object.id === this.state.objectId,
    );
  }

  get selectedVersion() {
    return this.selectedObject?.versions.find(
      (version) => version.versionId === this.state.versionId,
    );
  }

  get busy() {
    return (
      this.state.phase === "inspecting" || this.state.phase === "preparing"
    );
  }

  get canPrepare() {
    if (this.state.sourceMode === "files") {
      return Boolean(
        this.targetProjectId &&
        this.state.phase === "ready" &&
        this.state.fileSnapshot?.files.length,
      );
    }
    return (
      this.state.phase === "ready" &&
      Boolean(this.targetProjectId) &&
      this.state.sourceProjectId.trim() !== this.targetProjectId &&
      Boolean(this.selectedVersion?.sourceDigest)
    );
  }

  private update(change: Partial<ObjectImportState>) {
    this.state = { ...this.state, ...change };
    this.listeners.forEach((listener) => listener());
  }

  private invalidate() {
    this.revision += 1;
    this.request = undefined;
    this.fileRequest = undefined;
  }

  setSource(field: "path" | "sourceProjectId", value: string) {
    if (this.state[field] === value) return;
    this.selectSource(
      {
        path: field === "path" ? value : this.state.path,
        projectId: field === "sourceProjectId" ? value : "",
      },
      "external",
    );
  }

  selectSource(
    { path, projectId }: { path: string; projectId: string },
    mode: "project" | "external" = "project",
  ) {
    if (
      this.state.sourceMode === mode &&
      this.state.path === path &&
      this.state.sourceProjectId === projectId
    )
      return;
    this.invalidate();
    this.update({
      sourceMode: mode,
      path,
      sourceProjectId: projectId,
      selectedPaths: [],
      fileSnapshot: null,
      fileGroups: [],
      objects: [],
      objectId: "",
      versionId: "",
      phase: "editing",
      receipt: null,
      error: "",
    });
  }

  setSourceMode(mode: ObjectImportSourceMode) {
    if (mode === this.state.sourceMode) return;
    this.invalidate();
    this.update({
      sourceMode: mode,
      path: "",
      sourceProjectId: "",
      selectedPaths: [],
      fileSnapshot: null,
      fileGroups: [],
      objects: [],
      objectId: "",
      versionId: "",
      phase: "editing",
      receipt: null,
      error: "",
    });
  }

  addFilePaths(paths: readonly string[]) {
    const selected = new Set(this.state.selectedPaths);
    for (const path of paths) {
      const normalized = normalizeFilePath(path);
      if (normalized) selected.add(normalized);
    }
    const selectedPaths = [...selected];
    if (!selectedPaths.length && !this.state.selectedPaths.length) return;
    if (
      this.state.sourceMode === "files" &&
      selectedPaths.length === this.state.selectedPaths.length &&
      selectedPaths.every(
        (path, index) => path === this.state.selectedPaths[index],
      )
    )
      return;
    this.invalidate();
    this.update({
      sourceMode: "files",
      path: "",
      sourceProjectId: "",
      selectedPaths,
      fileSnapshot: null,
      fileGroups: this.state.fileGroups,
      objects: [],
      objectId: "",
      versionId: "",
      phase: "editing",
      receipt: null,
      error: "",
    });
  }

  selectFiles(paths: readonly string[]) {
    const selected = new Set<string>();
    for (const path of paths) {
      const normalized = normalizeFilePath(path);
      if (normalized) selected.add(normalized);
    }
    this.invalidate();
    this.update({
      sourceMode: "files",
      path: "",
      sourceProjectId: "",
      selectedPaths: [...selected],
      fileSnapshot: null,
      fileGroups: [],
      objects: [],
      objectId: "",
      versionId: "",
      phase: "editing",
      receipt: null,
      error: "",
    });
  }

  removeFilePath(path: string) {
    const normalized = normalizeFilePath(path);
    if (!this.state.selectedPaths.includes(normalized)) return;
    this.invalidate();
    const removedFiles = new Set(
      this.state.fileSnapshot?.files
        .filter(
          (file) => file.sourcePath === normalized || file.path === normalized,
        )
        .map((file) => file.path) ?? [],
    );
    removedFiles.add(normalized);
    const selectedPaths = this.state.selectedPaths.filter(
      (item) => item !== normalized,
    );
    this.update({
      selectedPaths,
      fileSnapshot: null,
      fileGroups: this.state.fileGroups.map((group) => ({
        ...group,
        paths: group.paths.filter((item) => !removedFiles.has(item)),
      })),
      objects: [],
      objectId: "",
      versionId: "",
      phase: "editing",
      receipt: null,
      error: "",
    });
  }

  private updateFileGroups(fileGroups: FileGroup[]) {
    if (fileGroups === this.state.fileGroups) return;
    this.invalidate();
    this.update({
      fileGroups,
      phase: this.state.fileSnapshot ? "ready" : "editing",
      receipt: null,
      error: "",
    });
  }

  addFileGroup(name: string) {
    this.updateFileGroups(addImportGroup(this.state.fileGroups, name));
  }

  removeFileGroup(id: string) {
    if (!this.state.fileGroups.some((group) => group.id === id)) return;
    this.updateFileGroups(
      this.state.fileGroups.filter((group) => group.id !== id),
    );
  }

  assignFileToGroup(path: string, groupId: string) {
    this.updateFileGroups(
      assignImportGroup(this.state, normalizeFilePath(path), groupId),
    );
  }

  selectObject(objectId: string) {
    const object = this.state.objects.find((item) => item.id === objectId);
    if (!object || objectId === this.state.objectId) return;
    this.invalidate();
    this.update({
      objectId,
      versionId: latest(object),
      phase: "ready",
      receipt: null,
      error: "",
    });
  }

  selectVersion(versionId: string) {
    if (
      versionId === this.state.versionId ||
      !this.selectedObject?.versions.some(
        (version) => version.versionId === versionId,
      )
    )
      return;
    this.invalidate();
    this.update({ versionId, phase: "ready", receipt: null, error: "" });
  }

  cancel(preserveSelection = false) {
    this.invalidate();
    this.update({
      phase: "editing",
      fileSnapshot: null,
      objects: [],
      objectId: preserveSelection ? this.state.objectId : "",
      versionId: preserveSelection ? this.state.versionId : "",
      receipt: null,
      error: "",
    });
  }

  async choosePaths(kind: "project" | "files" | "directory"): Promise<void> {
    if (this.busy) return;
    const revision = this.revision;
    const sequence = ++this.pickerSequence;
    const current = () =>
      revision === this.revision && sequence === this.pickerSequence;
    try {
      const result = await this.call(
        kind === "files" ? "chooseImportFiles" : "chooseImportDirectory",
        undefined,
      );
      if (!current()) return;
      const paths = kind === "files" ? result : result === null ? [] : [result];
      if (
        !Array.isArray(paths) ||
        !paths.every(
          (path: unknown): path is string =>
            typeof path === "string" && path.trim().length > 0,
        )
      )
        throw new Error("文件选择器返回的路径无效");
      if (!paths.length) return;
      if (kind === "project")
        this.selectSource({ path: paths[0]!, projectId: "" }, "external");
      else this.addFilePaths(paths);
      await this.inspect();
    } catch (error) {
      if (current()) this.update({ error: message(error) });
    }
  }

  async inspect(): Promise<void> {
    if (this.state.sourceMode === "files") {
      await this.inspectFiles();
      return;
    }
    const source = {
      path: this.state.path.trim(),
      ...(this.state.sourceProjectId.trim()
        ? { projectId: this.state.sourceProjectId.trim() }
        : {}),
    };
    if (this.busy || !source.path) return;
    const preferredObject = this.state.objectId;
    const preferredVersion = this.state.versionId;
    this.invalidate();
    const revision = this.revision;
    this.update({
      phase: "inspecting",
      objects: [],
      receipt: null,
      error: "",
    });
    try {
      const input = await this.call("object.inspectExternal", source);
      if (revision !== this.revision) return;
      const { projectId, objects } = parseImportInspection(
        input,
        source.projectId,
      );
      const object =
        objects.find((item) => item.id === preferredObject) ??
        objects.find((item) =>
          item.versions.some((version) => version.sourceDigest),
        ) ??
        objects[0];
      this.update({
        phase: "ready",
        path: source.path,
        sourceProjectId: projectId,
        objects,
        objectId: object?.id ?? "",
        versionId: object?.versions.some(
          (version) => version.versionId === preferredVersion,
        )
          ? preferredVersion
          : latest(object),
      });
    } catch (error) {
      if (revision === this.revision)
        this.update({ phase: "editing", error: message(error) });
    }
  }

  async inspectFiles(): Promise<void> {
    if (this.busy || !this.state.selectedPaths.length) return;
    this.invalidate();
    const revision = this.revision;
    const paths = [...this.state.selectedPaths];
    this.update({
      phase: "inspecting",
      fileSnapshot: null,
      objects: [],
      objectId: "",
      versionId: "",
      receipt: null,
      error: "",
    });
    try {
      const input = await this.call("object.inspectFiles", { paths });
      if (revision !== this.revision) return;
      const fileSnapshot = parseFileSourceSnapshot(input);
      const files = new Set(fileSnapshot.files.map((file) => file.path));
      this.update({
        phase: "ready",
        fileSnapshot,
        fileGroups: this.state.fileGroups.map((group) => ({
          ...group,
          paths: group.paths.filter((path) => files.has(path)),
        })),
      });
    } catch (error) {
      if (revision === this.revision)
        this.update({ phase: "editing", error: message(error) });
    }
  }

  async prepare(): Promise<
    ImportPreparationReceipt | FileImportPreparationReceipt | undefined
  > {
    if (this.state.sourceMode === "files") {
      if (!this.canPrepare || !this.targetProjectId || !this.state.fileSnapshot)
        return;
      const revision = ++this.revision;
      this.update({ phase: "preparing", receipt: null, error: "" });
      try {
        this.fileRequest ??= {
          requestId: this.requestId(),
          targetProjectId: this.targetProjectId,
          snapshot: this.state.fileSnapshot,
          groups: this.state.fileGroups.map((group) => ({
            ...group,
            paths: [...group.paths],
          })),
        };
        const request = this.fileRequest;
        const input = await this.call("object.prepareFileImport", request);
        if (revision !== this.revision) return;
        const receipt = parseFileImportReceipt(input, request);
        this.update({ phase: "prepared", receipt });
        return receipt;
      } catch (error) {
        if (revision === this.revision)
          this.update({ phase: "ready", error: message(error) });
      }
      return;
    }
    const sourceDigest = this.selectedVersion?.sourceDigest;
    if (!this.canPrepare || !this.targetProjectId || !sourceDigest) return;
    const revision = ++this.revision;
    this.update({ phase: "preparing", receipt: null, error: "" });
    try {
      this.request ??= {
        requestId: this.requestId(),
        targetProjectId: this.targetProjectId,
        source: {
          path: this.state.path.trim(),
          projectId: this.state.sourceProjectId.trim(),
        },
        objectId: this.state.objectId,
        baseline: { kind: "pinnedVersion", versionId: this.state.versionId },
        sourceDigest,
      };
      const request = this.request;
      const input = await this.call("object.prepareImport", request);
      if (revision !== this.revision) return;
      const receipt = parseImportReceipt(input, request);
      this.update({ phase: "prepared", receipt });
      return receipt;
    } catch (error) {
      if (revision === this.revision)
        this.update({ phase: "ready", error: message(error) });
    }
  }
}
