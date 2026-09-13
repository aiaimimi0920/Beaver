import type { Task, TaskExecutor } from "../shared/types";
import { CodexRpc, object } from "./rpc";
import { Preferences } from "./settings";
import { findTool } from "./process";
import { codeStructureInstructions, prepareHome } from "./codex-home";
import { formatTaskBrief } from "../shared/task-brief";
import { randomUUID } from "node:crypto";
import { askUserTool, questionsSchema } from "../shared/clarifications";
import {
  planTool,
  planningInstruction,
  taskPlanSchema,
} from "../shared/task-plan";

interface ActiveTask {
  rpc: CodexRpc;
  task: Task;
  timer?: NodeJS.Timeout;
  cancelled: boolean;
  started: Promise<void>;
  finished?: Promise<void>;
  finish: (status: string, error?: string) => Promise<void>;
}

export class LocalExecutor implements TaskExecutor {
  setAskRatio(
    id: string,
    value: import("../shared/autonomy").AskRatio | null,
  ): void {
    const entry = this.active.get(id);
    if (entry) entry.task.askRatio = value;
  }
  readonly location = "local" as const;
  private active = new Map<string, ActiveTask>();
  constructor(
    private root: string,
    private resources: string,
    private mcpScript: string,
    private prefs: Preferences,
    private event: (id: string, kind: string, text: string) => void,
    private changed: (task: Task) => void,
    private finished: (
      task: Task,
      status: string,
      error?: string,
    ) => Promise<void>,
    private rpcFactory: () => CodexRpc = () => new CodexRpc(),
  ) {}
  async start(task: Task): Promise<void> {
    let recoveryAttempts = 0;
    const rpc = this.rpcFactory();
    const finish = (status: string, error?: string): Promise<void> => {
      if (entry.finished) return entry.finished;
      entry.finished = Promise.resolve().then(async () => {
        clearTimeout(entry.timer);
        await rpc.close();
        await this.finished(
          task,
          entry.cancelled ? "interrupted" : status,
          error,
        );
      });
      void Promise.all([entry.started, entry.finished])
        .then(() => {
          if (this.active.get(task.id) === entry) this.active.delete(task.id);
        })
        .catch(() => undefined);
      return entry.finished;
    };
    const entry: ActiveTask = {
      rpc,
      task,
      cancelled: false,
      started: Promise.resolve(),
      finish,
    };
    this.active.set(task.id, entry);
    rpc.on(
      "serverRequest",
      (id: string | number, method: string, p: Record<string, unknown>) => {
        if (method === "item/tool/call" && p.tool === "beaver_submit_plan") {
          const parsed = taskPlanSchema.safeParse(p.arguments);
          if (
            entry.finished ||
            entry.cancelled ||
            !task.decompose ||
            task.parentTaskId ||
            task.plan ||
            p.threadId !== task.threadId ||
            (task.turnId && p.turnId !== task.turnId) ||
            task.clarifications?.some((q) => !q.answers) ||
            !parsed.success
          ) {
            rpc.rejectRequest(
              id,
              "Invalid, stale or duplicate executable plan",
            );
            return;
          }
          task.plan = parsed.data;
          task.report = parsed.data.summary;
          this.changed(task);
          rpc.respond(id, {
            success: true,
            contentItems: [
              { type: "inputText", text: "计划已保存，Beaver 将建立子任务。" },
            ],
          });
          void finish("completed");
          return;
        }
        const questionRequest =
          method === "item/tool/requestUserInput" ||
          (method === "item/tool/call" && p.tool === "beaver_ask_user");
        if (
          !questionRequest ||
          entry.finished ||
          entry.cancelled ||
          p.threadId !== task.threadId ||
          (task.turnId && p.turnId !== task.turnId)
        ) {
          rpc.rejectRequest(id, "Unsupported or stale request");
          return;
        }
        const parsed = questionsSchema.safeParse(
          method === "item/tool/call" ? p.arguments : p,
        );
        if (!parsed.success) {
          rpc.rejectRequest(id, "Ask 1-3 non-secret questions with unique IDs");
          return;
        }
        task.clarifications ??= [];
        const ratio = task.askRatio ?? this.prefs.read().askRatio ?? 100;
        const autoAnswers: Record<string, string> = {};
        const automaticQuestions: typeof parsed.data.questions = [];
        const manual: typeof parsed.data.questions = [];
        try {
          for (const q of parsed.data.questions) {
            const answer = automaticChoice(ratio, q);
            if (answer !== undefined) {
              autoAnswers[q.id] = answer;
              automaticQuestions.push(q);
            } else manual.push(q);
          }
        } catch (error) {
          rpc.rejectRequest(
            id,
            error instanceof Error
              ? error.message
              : "Invalid decision metadata",
          );
          return;
        }
        const item = {
          id: randomUUID(),
          createdAt: new Date().toISOString(),
          questions: manual,
          automaticQuestions,
          autoAnswers,
          askRatio: ratio,
          ...(manual.length
            ? {}
            : { answers: autoAnswers, answeredAt: new Date().toISOString() }),
        };
        task.clarifications.push(item);
        if (automaticQuestions.length)
          task.prompt +=
            "\n\n按自动决策设置采用（用户后续明确要求优先）：\n" +
            JSON.stringify(automaticQuestions);
        if (!manual.length) {
          this.changed(task);
          this.event(task.id, "decision", JSON.stringify(item));
          rpc.respond(
            id,
            method === "item/tool/call"
              ? {
                  success: true,
                  contentItems: [
                    {
                      type: "inputText",
                      text: JSON.stringify({
                        answers: autoAnswers,
                        source: "automatic",
                      }),
                    },
                  ],
                }
              : {
                  answers: Object.fromEntries(
                    Object.entries(autoAnswers).map(([k, v]) => [
                      k,
                      { answers: [v] },
                    ]),
                  ),
                },
          );
          return;
        }
        // Persist before closing the process. No fabricated answer is sent to Codex.
        task.status = "awaitingInput";
        this.changed(task);
        this.event(
          task.id,
          "question",
          parsed.data.questions.map((q) => q.question).join("\n"),
        );
        entry.cancelled = true;
        void finish("interrupted");
      },
    );
    rpc.on("log", (text: string) =>
      this.event(task.id, "system", this.prefs.redact(text)),
    );
    rpc.on("exit", () => {
      if (!entry.finished)
        void finish("failed", "Codex 进程意外退出；工作副本已保留。");
    });
    rpc.on("notification", (method: string, p: Record<string, unknown>) => {
      if (entry.finished || entry.cancelled) return;
      if (p.threadId && task.threadId && p.threadId !== task.threadId) return;
      if (method === "item/agentMessage/delta")
        this.event(
          task.id,
          "assistant",
          this.prefs.redact(String(p.delta ?? "")),
        );
      else if (method === "item/completed") {
        const item = object(p.item);
        const text =
          typeof item.text === "string" ? item.text : JSON.stringify(item);
        this.event(
          task.id,
          String(item.type ?? "item"),
          this.prefs.redact(text),
        );
        if (item.type === "agentMessage") task.report = this.prefs.redact(text);
      } else if (method === "turn/completed") {
        const turn = object(p.turn);
        const message = String(object(turn.error).message ?? "");
        if (
          !entry.cancelled &&
          !entry.finished &&
          turn.status === "failed" &&
          recoveryAttempts === 0 &&
          recoverableFailure(message)
        ) {
          recoveryAttempts++;
          this.event(
            task.id,
            "recovery",
            "已确认临时连接故障，自动恢复 1/1；检查已有结果后继续，不重放成功操作。",
          );
          void rpc
            .request("turn/start", {
              threadId: task.threadId,
              cwd: task.workspace,
              input: [
                {
                  type: "text",
                  text: "连接故障后先检查已有文件与操作结果，从未完成处继续，不重放已成功操作。",
                  text_elements: [],
                },
              ],
            })
            .then((result) => {
              if (entry.cancelled || entry.finished) return;
              task.turnId = String(object(object(result).turn).id);
              this.changed(task);
            })
            .catch((error) => void finish("failed", String(error)));
          return;
        }
        void finish(
          String(turn.status),
          String(object(turn.error).message ?? "") || undefined,
        );
      } else if (method === "turn/plan/updated")
        this.event(task.id, "plan", this.prefs.redact(JSON.stringify(p.plan)));
    });
    entry.started = (async () => {
      try {
        const command = await findTool("codex", this.prefs.read().tools.codex);
        const env = await prepareHome(
          this.root,
          this.resources,
          this.mcpScript,
          task,
          this.prefs,
        );
        if (entry.cancelled) return;
        await rpc.connect(command, env, task.workspace);
        if (entry.cancelled) return;
        const params = {
          cwd: task.workspace,
          approvalPolicy: "never",
          sandbox: "danger-full-access",
          model: this.prefs.resolve(task.capability).model,
          developerInstructions: await codeStructureInstructions(
            this.resources,
          ),
        };
        const result = object(
          await rpc.request(
            task.threadId ? "thread/resume" : "thread/start",
            task.threadId
              ? { ...params, threadId: task.threadId }
              : {
                  ...params,
                  dynamicTools: task.decompose
                    ? [askUserTool, planTool]
                    : [askUserTool],
                },
          ),
        );
        if (entry.cancelled) return;
        task.threadId = String(object(result.thread).id);
        task.turnId = undefined;
        this.changed(task);
        const text = [
          task.prompt,
          "资料使用可人工阅读和编辑的 Markdown，按需归档到 docs/world（世界观）、docs/characters（人物）、docs/mechanics（玩法）、docs/art（美术规范）、docs/audio（声音规范）、docs/production（制作计划）、docs/decisions（用户确认）。沿用已有项目的有效分类，不为凑目录生成空文档。由你自行检索并选择上下文。",
          "先阅读项目资料。缺少必须由用户决定的创作意图或知识时，调用 beaver_ask_user 提问；任务会等待回答，不要自行假设用户决定。收到回答后，将确认的设定归档到项目 docs/decisions 下的 Markdown 文档（仅审查任务不得写入）。",
          task.decompose === true
            ? planningInstruction
            : "这是一个独立阶段任务，不要再自动拆分新的子任务。",
          formatTaskBrief(task),
          "源代码、场景、配置和 Markdown 写为 UTF-8 无 BOM；PowerShell 使用 [IO.File]::WriteAllText(path, text, [Text.UTF8Encoding]::new($false))。验证前检查文件编码。明确失败的操作先检查当前状态，修正原因后最多重试两次；不重放已成功的操作。持续报告当前阶段和恢复结果。任务结束不代表交付通过，交付以 Beaver 标准导出及包验证为准。",
          `停止条件：${task.stopConditions || "完成目标后停止。遇到无法解决的授权或工具缺失时如实报告，不伪造成功。"}`,
          "参考素材（相对当前项目；区域为归一化坐标）：",
          JSON.stringify(task.references),
          task.capability === "review"
            ? "本任务只做审查，不修改项目。"
            : "在当前项目副本完成修改，不访问或修改原始项目。最后验证结果并用中文汇报。",
        ].join("\n\n");
        const response = object(
          await rpc.request("turn/start", {
            threadId: task.threadId,
            input: [{ type: "text", text, text_elements: [] }],
            cwd: task.workspace,
          }),
        );
        if (entry.cancelled) return;
        task.turnId = String(object(response.turn).id);
        this.changed(task);
        if (task.maxMinutes > 0)
          entry.timer = setTimeout(() => {
            this.event(task.id, "system", "达到任务时间限制，正在中止。");
            void this.interrupt(task.id);
          }, task.maxMinutes * 60000);
      } catch (error) {
        await finish("failed", this.prefs.redact(String(error)));
      }
    })();
    await entry.started;
  }
  async steer(id: string, text: string): Promise<void> {
    const e = this.active.get(id);
    if (!e?.task.threadId || !e.task.turnId || e.finished || e.cancelled)
      throw new Error("任务尚未就绪或已经结束，请稍后继续任务");
    e.task.prompt += "\n\n补充要求：" + text;
    this.changed(e.task);
    await e.rpc.request("turn/steer", {
      threadId: e.task.threadId,
      expectedTurnId: e.task.turnId,
      input: [{ type: "text", text, text_elements: [] }],
    });
  }
  async interrupt(id: string): Promise<void> {
    const e = this.active.get(id);
    if (!e) return;
    e.cancelled = true;
    if (e.task.threadId && e.task.turnId && !e.finished) {
      let timer: NodeJS.Timeout | undefined;
      try {
        await Promise.race([
          e.rpc.request("turn/interrupt", {
            threadId: e.task.threadId,
            turnId: e.task.turnId,
          }),
          new Promise<void>((resolve) => {
            timer = setTimeout(resolve, 1500);
          }),
        ]);
      } catch {
      } finally {
        clearTimeout(timer);
      }
    }
    await e.rpc.close();
    await e.started;
    await e.finish("interrupted");
    if (this.active.get(id) === e) this.active.delete(id);
  }
  async shutdown(): Promise<void> {
    await Promise.all([...this.active.keys()].map((id) => this.interrupt(id)));
  }
}
import { automaticChoice } from "../shared/autonomy";
import { recoverableFailure } from "../shared/recovery";
