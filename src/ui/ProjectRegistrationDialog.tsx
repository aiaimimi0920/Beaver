import { useEffect, useRef, useState } from "react";
import type { Project } from "../shared/types";
import { call, type Run } from "./api";
import { Dialog, Field } from "./components";
import { useNotify } from "./Notifications";
import { errorMessage } from "./notification-state";

export function ProjectRegistrationDialog({
  project,
  close,
  run,
}: {
  project: Project;
  close: () => void;
  run: Run;
}) {
  const notify = useNotify();
  const [path, setPath] = useState("");
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [inspection, setInspection] = useState(0);
  const [diagnostic, setDiagnostic] = useState("正在检查项目位置…");
  useEffect(() => {
    let active = true;
    setDiagnostic("正在检查项目位置…");
    void call<{ path: string; message: string }>("project.storage.status", {
      id: project.id,
    })
      .then((result) => {
        if (active)
          setDiagnostic(
            result.path === project.path
              ? result.message
              : "项目登记路径已改变，请关闭此窗口并刷新后重试。",
          );
      })
      .catch((error: unknown) => {
        if (active) setDiagnostic(`检查失败：${errorMessage(error)}`);
      });
    return () => {
      active = false;
    };
  }, [project.id, project.path, inspection]);
  function perform(work: () => Promise<unknown>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    void run(work).finally(() => {
      pending.current = false;
      setBusy(false);
    });
  }
  return (
    <Dialog title="管理项目登记" close={busy ? () => {} : close}>
      <p>{project.name}</p>
      <Field label="当前登记路径">
        <input aria-label="当前登记路径" value={project.path} readOnly />
      </Field>
      <p role="status">{diagnostic}</p>
      <button
        disabled={busy}
        onClick={() => setInspection((value) => value + 1)}
      >
        重新检查位置
      </button>
      <p className="muted">
        项目移动后可重新关联同一项目目录。Beaver
        会核对项目身份；请先结束活动任务。
      </p>
      <Field label="新的项目目录">
        <div className="input-action">
          <input
            aria-label="新的项目目录"
            value={path}
            disabled={busy}
            onChange={(event) => setPath(event.target.value)}
          />
          <button
            disabled={busy}
            onClick={() =>
              perform(async () => {
                const selected = await call<string | null>("chooseDirectory");
                if (selected) setPath(selected);
              })
            }
          >
            选择目录
          </button>
        </div>
      </Field>
      <button
        disabled={busy || !path.trim() || path.trim() === project.path}
        onClick={() =>
          perform(async () => {
            await call("project.reassociate", {
              id: project.id,
              expectedPath: project.path,
              path: path.trim(),
            });
            notify({ tone: "success", text: "项目目录已重新关联" });
            close();
          })
        }
      >
        重新关联
      </button>
      <p className="muted">
        从列表移除只注销本机登记，保留项目文件与历史。已开始的工作继续收尾，不再领取新工作。
      </p>
      {confirmRemoval && (
        <p role="alert">确认从本机项目列表移除“{project.name}”？</p>
      )}
      <footer>
        <button disabled={busy} onClick={close}>
          关闭
        </button>
        {confirmRemoval ? (
          <>
            <button disabled={busy} onClick={() => setConfirmRemoval(false)}>
              取消移除
            </button>
            <button
              disabled={busy}
              onClick={() =>
                perform(async () => {
                  const result = await call<{ draining: boolean }>(
                    "project.unregister",
                    {
                      id: project.id,
                      expectedPath: project.path,
                    },
                  );
                  notify({
                    tone: "success",
                    text: result.draining
                      ? "已移除项目登记，已开始的工作仍在收尾"
                      : "已移除项目登记，项目文件与历史已保留",
                  });
                  close();
                })
              }
            >
              确认移除
            </button>
          </>
        ) : (
          <button disabled={busy} onClick={() => setConfirmRemoval(true)}>
            从列表移除
          </button>
        )}
      </footer>
    </Dialog>
  );
}
