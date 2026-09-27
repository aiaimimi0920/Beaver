import {
  objectAttemptCheckRequestSchema,
  type ObjectAttemptCheckReport,
  type ObjectAttemptCheckRequest,
} from "../../shared/object-attempt-checks";
import {
  validationObjectReportSchema,
  type ValidationObjectSource,
} from "../../shared/validation-object-report";

interface State {
  busy: boolean;
  error: string;
  report?: ObjectAttemptCheckReport;
  source?: ValidationObjectSource;
}

export class ObjectCheckReportQuery {
  readonly request: ObjectAttemptCheckRequest;
  private generation = 0;
  private state: State = { busy: false, error: "" };
  private listeners = new Set<() => void>();
  constructor(
    projectId: string,
    request: ObjectAttemptCheckRequest,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
  ) {
    this.request = objectAttemptCheckRequestSchema.parse(request);
    if (this.request.projectId !== projectId)
      throw new Error("技术报告不属于当前项目");
  }
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(state: State) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
    this.set({ ...this.state, busy: false });
  };
  refresh = async () => {
    if (this.state.busy) return;
    const generation = ++this.generation;
    this.set({ busy: true, error: "" });
    if (generation !== this.generation) return;
    try {
      const response = await this.api("validation.objectReport.get", {
        projectId: this.request.projectId,
        attemptId: this.request.target.attemptId,
        requestId: this.request.requestId,
      });
      if (generation !== this.generation) return;
      const { source, report } = validationObjectReportSchema.parse(response);
      if (
        report.request.projectId !== this.request.projectId ||
        JSON.stringify(report.request.target) !==
          JSON.stringify(this.request.target) ||
        report.request.requestId !== this.request.requestId
      )
        throw new Error("指定技术报告来源不一致；不会替换为其他轮次或最新报告");
      this.set({ busy: false, error: "", report, source });
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, error: String(error) });
    }
  };
}
