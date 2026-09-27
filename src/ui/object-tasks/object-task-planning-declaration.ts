import {
  planningDeclarationRequestSchema,
  planningDeclarationSchema,
  type PlanningDeclarationRequest,
  type PlanningState,
} from "../../shared/object-task-planning-declaration";

type Call = (method: string, input: unknown) => Promise<unknown>;

// A lost response retries the same payload even after a background refresh.
export class PlanningDeclarationSubmission {
  private request: PlanningDeclarationRequest | undefined;
  private busy = false;
  constructor(
    private readonly projectId: string,
    private readonly taskId: string,
    private readonly call: Call,
    private readonly id: () => string = () => crypto.randomUUID(),
  ) {}
  get pending() {
    return this.request !== undefined;
  }
  reset() {
    if (!this.busy) this.request = undefined;
  }
  async submit(state: PlanningState, planRevision: number, reason: string) {
    if (this.busy) return false;
    this.busy = true;
    try {
      if (!this.request) {
        if (
          state.taskId !== this.taskId ||
          state.blockers.length ||
          state.current
        )
          throw new Error("当前范围无法声明，请刷新任务后检查规划缺口。");
        this.request = planningDeclarationRequestSchema.parse({
          projectId: this.projectId,
          taskId: this.taskId,
          requestId: this.id(),
          expectedPlanRevision: planRevision,
          expectedScopeHash: state.scopeHash,
          reason,
        });
      }
      const receipt = planningDeclarationSchema.parse(
        await this.call("objectTask.declarePlanningComplete", this.request),
      );
      if (JSON.stringify(receipt.request) !== JSON.stringify(this.request))
        throw new Error("规划声明回执与请求不一致，请重试查询原结果。");
      this.request = undefined;
      return true;
    } finally {
      this.busy = false;
    }
  }
}
