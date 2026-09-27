import {
  objectAttemptCheckReportSchema,
  objectAttemptCheckRequestSchema,
  type ObjectAttemptCheckReport,
  type ObjectAttemptCheckRequest,
} from "../../shared/object-attempt-checks";
import type { ObjectExecution } from "../../shared/object-attempts";

interface State {
  busy: boolean;
  error: string;
  retry: boolean;
  reports: ObjectAttemptCheckReport[];
}

export class ObjectAttemptChecks {
  private generation = 0;
  private request: ObjectAttemptCheckRequest | null = null;
  private listeners = new Set<() => void>();
  private state: State = { busy: false, error: "", retry: false, reports: [] };
  private readonly target: ObjectAttemptCheckRequest["target"];
  constructor(
    private readonly projectId: string,
    attempt: ObjectExecution["attempt"],
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
    private readonly requestId: () => string = () => crypto.randomUUID(),
  ) {
    if (
      attempt.projectId !== projectId ||
      !attempt.outputCaptured ||
      attempt.state === "running"
    )
      throw new Error("技术检查需要当前项目的冻结输出");
    this.target = objectAttemptCheckRequestSchema.parse({
      projectId,
      requestId: "validate",
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
  private matches(report: ObjectAttemptCheckReport) {
    if (
      report.request.projectId !== this.projectId ||
      JSON.stringify(report.request.target) !== JSON.stringify(this.target)
    )
      throw new Error("技术报告与当前尝试不一致");
  }
  refresh = async () => {
    if (this.state.busy) return;
    const generation = ++this.generation;
    this.set({ busy: true, error: "" });
    if (generation !== this.generation) return;
    try {
      const response = await this.api("objectTask.attemptChecks", {
        projectId: this.projectId,
        attemptId: this.target.attemptId,
      });
      if (generation !== this.generation) return;
      const reports = objectAttemptCheckReportSchema.array().parse(response);
      const ids = new Set<string>();
      for (const report of reports) {
        this.matches(report);
        if (ids.has(report.request.requestId))
          throw new Error("技术报告请求 ID 重复");
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
  run = async () => {
    if (this.state.busy) return;
    this.request ??= objectAttemptCheckRequestSchema.parse({
      projectId: this.projectId,
      target: this.target,
      requestId: this.requestId(),
    });
    const generation = ++this.generation;
    this.set({ busy: true, error: "", retry: false });
    if (generation !== this.generation) return;
    try {
      const response = await this.api(
        "objectTask.checkAttempt",
        structuredClone(this.request),
      );
      if (generation !== this.generation) return;
      const report = objectAttemptCheckReportSchema.parse(response);
      this.matches(report);
      if (JSON.stringify(report.request) !== JSON.stringify(this.request))
        throw new Error("技术报告与检查请求不一致");
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
