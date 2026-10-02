import { call } from "./api";
import { Field } from "./components";
import { ProjectDerivationAssembly } from "./ProjectDerivationAssembly";
import {
  useProjectDerivation,
  type DerivationMode,
} from "./project-derivation-session";

export function ProjectDerivationPanel({
  busy,
  perform,
  openProject,
}: {
  busy: boolean;
  perform: (work: () => Promise<void>) => Promise<void>;
  openProject?: (id: string) => Promise<void>;
}) {
  const s = useProjectDerivation();
  return (
    <section aria-label="独立项目副本">
      <h3>派生独立项目副本</h3>
      <p className="muted">
        源项目必须离线。保留源项目和准备副本，生成全新项目身份，再显式启用、登记。支持旧项目记录，以及对象目录、父子关系、固定版本引用、文件快照、接受历史、从未领取执行的制造计划（含草稿、修订、已入队顺序、暂停及执行前撤销历史），以及非运行中的
        AI
        规划会话。规划候选可继续显式采纳，待回答会话可显式回答；读取、重开或重放旧请求不会自动启动
        AI
        或旧任务。副本中的排队任务默认暂停，打开、重开或登记重试不会解除；从未执行的排队任务需在制造计划中明确恢复才可执行。规划完成声明保留确认依据和历史覆盖范围，按新身份重算范围指纹；有效或失效状态保持，可在副本中显式重新确认，但不代表任务验收。
      </p>
      <p className="muted">
        另支持完整的停止执行链：失败或中断后的纯重试、按顺序显式接受并跨细修推进的历史，以及最终细修的一次或多次纯文本候选返工历史；最后一次可为失败、中断或待验收。输出须已捕获，工作区和历史证据须完整。保留已接受细修、推进回执、返工回执、冻结输入、各次输出、检查、中断和核验历史，并默认安全暂停。解除暂停不是恢复执行，必须在任务恢复界面重新核验；失败或中断时显式恢复，待验收时检查候选后显式接受并推进下一细修，或在最终细修新建候选审阅并明确返工，才会创建一次新的尝试。不会复用旧线程自动续跑。旧暂停/恢复、排序、核验和显式执行请求仅重放历史，不解除副本的新暂停，也不会启动任务。
      </p>
      <p className="muted">
        最终细修到达待验收、此前细修均已按序接受时形成的整体候选审阅（含旧版记录），在后续返工停止后仍可保留，但对象目录成员必须未变，历史对象版本须有完整回执。原时间、检查结论和用户反馈原文保持；只转换系统生成反馈尾缀中的身份引用，并重新计算新身份相关摘要。历史审阅不是新的返工授权：解除安全暂停后，待验收候选需重新核验、技术检查并生成新审阅，再从新记录填写反馈、显式确认返工。
      </p>
      <p className="muted">
        源项目中仅领取而没有首次停止尝试、运行中、包含图片或预览帧或区域重定位的返工历史、顺序不明的并列细修，以及处置、导入、发布操作和运行中的
        AI 规划等暂不支持记录的项目仍会在准备前拒绝，不会丢弃未知记录。
      </p>
      <Field label="派生操作">
        <select
          aria-label="派生操作"
          disabled={busy}
          value={s.mode}
          onChange={(event) =>
            s.changeMode(event.target.value as DerivationMode)
          }
        >
          <option value="new">从离线项目新建独立副本</option>
          <option value="prepared">继续已有准备副本（含响应丢失）</option>
          <option value="registered">重试已启用副本登记</option>
        </select>
      </Field>
      {s.mode === "new" && (
        <>
          <Field label="离线源项目目录">
            <div className="input-action">
              <input
                aria-label="离线源项目目录"
                disabled={busy || !!s.prepared}
                value={s.source}
                onChange={(event) => s.changeSource(event.target.value)}
              />
              <button
                disabled={busy || !!s.prepared}
                onClick={() =>
                  void perform(async () => {
                    const selected = await call<string | null>(
                      "chooseDirectory",
                    );
                    if (selected) s.changeSource(selected);
                  })
                }
              >
                选择源项目
              </button>
            </div>
          </Field>
          <button
            disabled={busy || !s.source.trim() || !!s.prepared}
            onClick={() => void perform(s.inspectSource)}
          >
            检查派生源项目
          </button>
          {s.inspected && (
            <p role="status">
              源项目 ID：{s.inspected.request.sourceProjectId}；检查到{" "}
              {s.inspected.entities} 条实体、{s.inspected.calls}{" "}
              条调用记录。拟分配新 ID：{s.inspected.request.targetProjectId}
              。准备时会重新检查源项目。
            </p>
          )}
        </>
      )}
      <Field label="组装准备目录">
        <input
          aria-label="组装准备目录"
          disabled={busy}
          value={s.preparation}
          onChange={(event) => s.changePreparation(event.target.value)}
        />
      </Field>
      <p className="muted">
        新建时填写尚不存在的绝对路径，父目录须存在。恢复时填写包含
        DERIVATION-COPY.json 的目录。
      </p>
      {s.mode !== "registered" && (
        <>
          {s.mode === "new" && (
            <button
              disabled={
                busy ||
                !s.inspected ||
                !s.preparation.trim() ||
                !!s.prepared ||
                s.preparation.trim() === s.failedPreparation
              }
              onClick={() => void perform(s.prepare)}
            >
              准备独立副本
            </button>
          )}
          <button
            disabled={
              busy ||
              !s.preparation.trim() ||
              s.registrationAttempted ||
              !!s.assembly?.activated
            }
            onClick={() => void perform(s.inspectPreparation)}
          >
            核验已有准备副本
          </button>
        </>
      )}
      {s.failedPreparation && !s.prepared && (
        <p role="status">
          准备结果未知或失败，保留 {s.failedPreparation}
          。先核验已有回执；不完整时换一个全新准备目录，不能覆盖重试。
        </p>
      )}
      {s.prepared && (
        <p role="status">
          准备已核验：{s.prepared.request.sourceProjectId} →{" "}
          {s.prepared.request.targetProjectId}。原请求{" "}
          {s.prepared.request.requestId}{" "}
          已保存在回执中；原源项目不可用时仍可继续。
        </p>
      )}
      <Field label="组装目标目录">
        <input
          aria-label="组装目标目录"
          disabled={busy}
          value={s.destination}
          onChange={(event) => s.changeDestination(event.target.value)}
        />
      </Field>
      <p className="muted">
        组装时也需要全新目录；它必须在源项目和准备目录之外。成功后项目位于此目录的
        project 子目录。关闭弹窗不会清理任何副本，可重新输入这两个路径继续。
      </p>
      <ProjectDerivationAssembly
        session={s}
        busy={busy}
        perform={perform}
        openProject={openProject}
      />
    </section>
  );
}
