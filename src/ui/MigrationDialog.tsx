import { useRef, useState } from "react";
import { call } from "./api";
import { Dialog, Field } from "./components";
import { errorMessage } from "./notification-state";
import { ProjectDerivationPanel } from "./ProjectDerivationPanel";

type Counts = { entities: number; events: number; calls: number };
type Inventory = { projects: Record<string, Counts>; unresolved: Counts };
type Receipt = { restoredData: string };
type Activation = {
  ready_to_activate: boolean;
  data_directory: string;
  default_data_directory_changed: boolean;
};
export function MigrationDialog({
  close,
  openProject,
}: {
  close: () => void;
  openProject?: (id: string) => Promise<void>;
}) {
  const [backup, setBackup] = useState("");
  const [target, setTarget] = useState("");
  const [tools, setTools] = useState("");
  const [resume, setResume] = useState(false);
  const [inventory, setInventory] = useState<Inventory>();
  const [receipt, setReceipt] = useState<Receipt>();
  const [activation, setActivation] = useState<Activation>();
  const [failedTarget, setFailedTarget] = useState("");
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  async function perform(work: () => Promise<void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError("");
    try {
      await work();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  const dismiss = () => {
    if (!pending.current) close();
  };
  const frozen = busy || !!receipt || !!activation;
  const canActivate = !!inventory && !!target.trim() && (!!receipt || resume);
  return (
    <Dialog title="数据迁移" close={dismiss}>
      <p className="muted">
        选择已完成的离线迁移归档。原数据与归档保留，准备只写入新副本；不会自动切换当前环境。
      </p>
      <Field label="归档目录">
        <div className="input-action">
          <input
            aria-label="归档目录"
            value={backup}
            disabled={frozen}
            onChange={(event) => {
              setBackup(event.target.value);
              setInventory(undefined);
              setConfirm(false);
            }}
          />
          <button
            disabled={frozen}
            onClick={() =>
              void perform(async () => {
                const selected = await call<string | null>("chooseDirectory");
                if (selected) {
                  setBackup(selected);
                  setInventory(undefined);
                  setConfirm(false);
                }
              })
            }
          >
            选择归档
          </button>
        </div>
      </Field>
      <button
        disabled={frozen || !backup.trim()}
        onClick={() =>
          void perform(async () => {
            setInventory(undefined);
            setConfirm(false);
            setInventory(
              await call<Inventory>("migration.inspect", {
                backup: backup.trim(),
              }),
            );
          })
        }
      >
        检查归档
      </button>
      {inventory && (
        <p role="status">
          检查完成：{Object.keys(inventory.projects).length} 个项目；未归属记录{" "}
          {inventory.unresolved.entities +
            inventory.unresolved.events +
            inventory.unresolved.calls}{" "}
          条。 未归属历史可能保留在宿主副本，不能自动投入执行。
        </p>
      )}
      <label className="field">
        <span>副本操作</span>
        <select
          aria-label="副本操作"
          value={resume ? "resume" : "new"}
          disabled={frozen}
          onChange={(event) => {
            setResume(event.target.value === "resume");
            setTarget("");
            setConfirm(false);
          }}
        >
          <option value="new">准备全新副本</option>
          <option value="resume">启用已有的已分区副本</option>
        </select>
      </label>
      <Field
        label={resume ? "已分区副本目录" : "全新副本目录（父目录须已存在）"}
      >
        <input
          aria-label="副本目录"
          value={target}
          disabled={frozen}
          onChange={(event) => {
            setTarget(event.target.value);
            setConfirm(false);
          }}
        />
      </Field>
      {!resume && !receipt && (
        <button
          disabled={
            busy ||
            !inventory ||
            !target.trim() ||
            target.trim() === failedTarget
          }
          onClick={() =>
            void perform(async () => {
              const destination = target.trim();
              try {
                setReceipt(
                  await call<Receipt>("migration.prepareProjects", {
                    backup: backup.trim(),
                    destination,
                  }),
                );
              } catch (error) {
                setFailedTarget(destination);
                throw error;
              }
            })
          }
        >
          准备副本
        </button>
      )}
      {failedTarget && !receipt && (
        <p role="status">
          准备失败，可能的部分副本保留在 {failedTarget}
          。重新准备请选择新的目标目录。
        </p>
      )}
      {receipt && !activation && (
        <p role="status">
          分区准备完成，尚未启用。数据目录：{receipt.restoredData}
          。启用前将检查路径、凭据和工具；保留的旧历史需另行转换。
        </p>
      )}
      {resume && (
        <p className="muted">
          选择包含 PROJECT-MIGRATION.json
          的副本根目录。仅恢复、未完成分区的副本不能启用。
        </p>
      )}
      {!activation && (
        <>
          <Field label="工具路径 JSON 文件（可选，绝对路径）">
            <input
              aria-label="工具路径 JSON 文件"
              value={tools}
              disabled={busy}
              onChange={(event) => {
                setTools(event.target.value);
                setConfirm(false);
              }}
            />
          </Field>
          <p className="muted">
            留空使用副本中的工具设置。启用会运行工具探测，失败仍保留
            pending，不发起模型请求。
          </p>
          {confirm ? (
            <>
              <p role="alert">
                确认启用副本 {target.trim()}？成功后仍需退出当前
                Beaver，并显式选择副本的数据目录启动。
              </p>
              <button disabled={busy} onClick={() => setConfirm(false)}>
                取消启用
              </button>
              <button
                disabled={busy || !canActivate}
                onClick={() =>
                  void perform(async () => {
                    const result = await call<Activation>(
                      "migration.activate",
                      {
                        backup: backup.trim(),
                        prepared: target.trim(),
                        ...(tools.trim() ? { toolPaths: tools.trim() } : {}),
                      },
                    );
                    if (
                      !result.ready_to_activate ||
                      result.default_data_directory_changed
                    )
                      throw new Error(
                        "启用回执异常，请保留副本并检查 ACTIVATION.json。",
                      );
                    setActivation(result);
                    setConfirm(false);
                  })
                }
              >
                确认启用副本
              </button>
            </>
          ) : (
            <button
              disabled={busy || !canActivate}
              onClick={() => setConfirm(true)}
            >
              启用副本
            </button>
          )}
        </>
      )}
      {activation && (
        <p role="status">
          副本已启用，当前环境未切换。关闭当前 Beaver 后，将 BEAVER_DATA_DIR
          设置为 {activation.data_directory}{" "}
          再启动。请保留原环境用于回退，勿让旧 Electron 读取转换后的副本。
        </p>
      )}
      <ProjectDerivationPanel
        busy={busy}
        perform={perform}
        openProject={openProject}
      />
      {busy && <p role="status">正在处理，请勿退出应用或修改迁移目录…</p>}
      {error && <p role="alert">{error}</p>}
      <footer>
        <button disabled={busy} onClick={dismiss}>
          关闭
        </button>
      </footer>
    </Dialog>
  );
}
