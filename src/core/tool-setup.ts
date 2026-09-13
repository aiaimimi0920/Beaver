import type { ToolName, ToolSetupState } from "../shared/tool-setup";
import type { ToolStatus } from "../shared/types";

export class ToolSetup {
  private controller?: AbortController;
  private state: ToolSetupState;
  constructor(
    private operations: {
      detect: (name: ToolName, signal: AbortSignal) => Promise<ToolStatus>;
      install: (name: ToolName, signal: AbortSignal) => Promise<void>;
      accept: (tool: ToolStatus) => void;
      save: (state: ToolSetupState) => void;
      redact: (error: string) => string;
    },
    previous?: ToolSetupState,
  ) {
    this.state = previous ?? { status: "idle", updatedAt: "", steps: [] };
    if (this.state.status === "running")
      this.update({
        status: "cancelled",
        error: "上次环境准备已中断；重新准备会保留已安装工具。",
        active: undefined,
        steps: this.state.steps.map((step) =>
          ["checking", "installing"].includes(step.status)
            ? { ...step, status: "cancelled" }
            : step,
        ),
      });
  }
  read(): ToolSetupState {
    return structuredClone(this.state);
  }
  cancel(): void {
    this.controller?.abort();
  }
  private update(patch: Partial<ToolSetupState>) {
    this.state = {
      ...this.state,
      ...patch,
      updatedAt: new Date().toISOString(),
    };
    this.operations.save(this.read());
  }
  async prepare(
    names: ToolName[] = ["node", "codex", "godot", "blender"],
  ): Promise<ToolSetupState> {
    if (this.controller) throw new Error("已有环境准备正在进行");
    const controller = new AbortController();
    this.controller = controller;
    const order = [
      ...new Set(names.includes("codex") ? ["node" as const, ...names] : names),
    ];
    try {
      this.update({
        status: "running",
        active: undefined,
        error: undefined,
        steps: order.map((name) => ({ name, status: "waiting" })),
      });
      for (const step of this.state.steps) {
        controller.signal.throwIfAborted();
        step.status = "checking";
        this.update({ active: step.name });
        let result = await this.operations.detect(step.name, controller.signal);
        controller.signal.throwIfAborted();
        if (!result.available) {
          step.status = "installing";
          this.update({});
          await this.operations.install(step.name, controller.signal);
          controller.signal.throwIfAborted();
          step.status = "checking";
          this.update({});
          result = await this.operations.detect(step.name, controller.signal);
        }
        controller.signal.throwIfAborted();
        if (!result.available || !result.path)
          throw new Error(
            `${step.name} 安装后仍未通过版本检测：${result.version}`,
          );
        this.operations.accept(result);
        step.status = "ready";
        step.result = result;
        this.update({});
      }
      this.update({ status: "completed", active: undefined });
    } catch (error) {
      const step = this.state.steps.find(
        (item) => item.name === this.state.active,
      );
      if (step)
        step.status = controller.signal.aborted ? "cancelled" : "failed";
      this.update({
        status: controller.signal.aborted ? "cancelled" : "failed",
        error: controller.signal.aborted
          ? "已停止准备；已安装的工具会保留。"
          : this.operations.redact(String(error)).slice(-4000),
        active: undefined,
      });
    } finally {
      this.controller = undefined;
    }
    return this.read();
  }
}
