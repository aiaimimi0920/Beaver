import {
  objectCandidateReportSchema,
  objectCandidateRequestSchema,
  type ObjectCandidateReport,
  type ObjectCandidateRequest,
} from "../../shared/object-candidate-review";
import type { ObjectExecution } from "../../shared/object-attempts";

interface State {
  busy: boolean;
  retry: boolean;
  error: string;
  reports: ObjectCandidateReport[];
}

export class ObjectCandidateReview {
  private generation = 0;
  private request: ObjectCandidateRequest | null = null;
  private listeners = new Set<() => void>();
  private state: State = { busy: false, retry: false, error: "", reports: [] };
  private target: ObjectCandidateRequest["target"];
  constructor(
    private projectId: string,
    attempt: ObjectExecution["attempt"],
    private api: (method: string, input: unknown) => Promise<unknown>,
    private requestId: () => string = () => crypto.randomUUID(),
  ) {
    if (
      attempt.projectId !== projectId ||
      attempt.state !== "awaitingGate" ||
      !attempt.outputCaptured
    )
      throw new Error("整体候选审阅需要已完成的冻结输出");
    this.target = objectCandidateRequestSchema.parse({
      projectId,
      requestId: "validate",
      checkRequestId: "validate",
      target: attempt.target,
    }).target;
  }
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
    this.set({ busy: false, retry: !!this.request });
  };
  private matches(report: ObjectCandidateReport) {
    if (
      report.request.projectId !== this.projectId ||
      JSON.stringify(report.request.target) !== JSON.stringify(this.target)
    )
      throw new Error("候选审阅记录与当前尝试不一致");
  }
  refresh = async () => {
    if (this.state.busy) return;
    const generation = ++this.generation;
    this.set({ busy: true, error: "" });
    if (generation !== this.generation) return;
    try {
      const response = await this.api("objectTask.candidateReviews", {
        projectId: this.projectId,
        attemptId: this.target.attemptId,
      });
      if (generation !== this.generation) return;
      const reports = objectCandidateReportSchema.array().parse(response);
      const ids = new Set<string>();
      for (const report of reports) {
        this.matches(report);
        if (ids.has(report.request.requestId))
          throw new Error("候选审阅请求 ID 重复");
        ids.add(report.request.requestId);
      }
      if (
        this.request &&
        reports.some(
          (report) =>
            JSON.stringify(report.request) === JSON.stringify(this.request),
        )
      )
        this.request = null;
      this.set({ reports, busy: false, retry: !!this.request });
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, error: String(error) });
    }
  };
  prepare = async (checkRequestId?: string) => {
    if (this.state.busy || (!this.request && !checkRequestId)) return;
    this.request ??= objectCandidateRequestSchema.parse({
      projectId: this.projectId,
      requestId: this.requestId(),
      target: this.target,
      checkRequestId,
    });
    const generation = ++this.generation;
    this.set({ busy: true, retry: false, error: "" });
    if (generation !== this.generation) return;
    try {
      const response = await this.api(
        "objectTask.prepareCandidateReview",
        structuredClone(this.request),
      );
      if (generation !== this.generation) return;
      const report = objectCandidateReportSchema.parse(response);
      this.matches(report);
      if (JSON.stringify(report.request) !== JSON.stringify(this.request))
        throw new Error("候选审阅回执与当前请求不一致");
      this.request = null;
      this.set({
        busy: false,
        retry: false,
        reports: [
          ...this.state.reports.filter(
            (old) => old.request.requestId !== report.request.requestId,
          ),
          report,
        ],
      });
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, retry: true, error: String(error) });
    }
  };
}
