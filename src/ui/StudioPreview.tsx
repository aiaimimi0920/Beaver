import { useEffect, useRef, useState } from "react";
import { StudioDocuments } from "./StudioDocuments";
import { StudioTasks } from "./StudioTasks";
import {
  createStudioState,
  studioSchema,
  type StudioPage,
} from "./studio-preview";
import { useNotify } from "./Notifications";
import { Dialog } from "./components";
import "./studio.css";

export function StudioPreview({
  projectKey,
  page,
  navigate,
  existing,
}: {
  projectKey: string;
  page: StudioPage;
  navigate: (page: StudioPage) => void;
  existing?: (page: "tasks") => void;
}) {
  const key = `beaver.studio-preview.v1.${projectKey}`;
  const [state, setState] = useState(() => {
    try {
      const saved = studioSchema.safeParse(
        JSON.parse(localStorage.getItem(key) ?? "null"),
      );
      if (saved.success) return saved.data;
    } catch {
      /* No project data is involved in the preview. */
    }
    return createStudioState();
  });
  const [selected, setSelected] = useState(
    state.tasks.find((t) => t.status === "需要你决定")?.id ??
      state.tasks[0]?.id ??
      "",
  );
  const [reset, setReset] = useState(false);
  const notify = useNotify();
  const warned = useRef(false);
  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(state));
    } catch {
      if (!warned.current) {
        warned.current = true;
        notify({ tone: "warning", text: "预览草稿未能保存，关闭后可能丢失。" });
      }
    }
  }, [key, state, notify]);
  const request = (goal: string, refs: string[]) => {
    const id = crypto.randomUUID();
    setState((old) => ({
      ...old,
      tasks: [{ id, goal, refs, status: "执行中", messages: [] }, ...old.tasks],
    }));
    setSelected(id);
    navigate("create");
  };
  return (
    <div className="studio-preview">
      <div className="studio-preview-bar">
        <span>交互预览 · 示例数据，不执行任务或写入项目</span>
        <button onClick={() => setReset(true)}>重置预览</button>
      </div>
      {page === "docs" ? (
        <StudioDocuments
          notes={state.notes}
          change={(notes) => setState((old) => ({ ...old, notes }))}
          request={request}
        />
      ) : (
        <StudioTasks
          tasks={state.tasks}
          selected={selected}
          select={setSelected}
          update={(task) =>
            setState((old) => ({
              ...old,
              tasks: old.tasks.map((t) => (t.id === task.id ? task : t)),
            }))
          }
          create={request}
          existing={existing ? () => existing("tasks") : undefined}
        />
      )}
      {reset && (
        <Dialog title="重置交互预览" close={() => setReset(false)}>
          <p>将清除预览中的任务与文档草稿，不影响真实项目。</p>
          <footer>
            <button onClick={() => setReset(false)}>取消</button>
            <button
              className="danger"
              onClick={() => {
                setState(createStudioState());
                setSelected("load");
                setReset(false);
              }}
            >
              重置
            </button>
          </footer>
        </Dialog>
      )}
    </div>
  );
}
