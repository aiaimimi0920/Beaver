import { useEffect, useRef, useState } from "react";
import type { FrameworkJob, FrameworkState } from "../../shared/framework";
import { call, type Run } from "../api";
import { errorMessage } from "../notification-state";

export function useFramework(id: string, run: Run) {
  const [data, setData] = useState<FrameworkState | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const retry = useRef({ signature: "", id: "" });
  const generation = useRef(0);
  const sequence = useRef(0);
  const mutating = useRef(false);
  const request = (input: unknown) =>
    call<FrameworkState>("task.framework", { id, request: input });
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    generation.current += 1;
    setData(null);
    setBusy(false);
    setError("");
    mutating.current = false;
    retry.current = { signature: "", id: "" };
    const refresh = async () => {
      if (mutating.current) {
        if (!disposed) timer = setTimeout(() => void refresh(), 1500);
        return;
      }
      const read = ++sequence.current;
      try {
        const next = await call<FrameworkState>("task.framework", {
          id,
          request: { operation: "state" },
        });
        if (!disposed && read === sequence.current) {
          setData(next);
          setError("");
        }
      } catch (failure: unknown) {
        if (!disposed && read === sequence.current)
          setError(errorMessage(failure));
      } finally {
        if (!disposed) timer = setTimeout(() => void refresh(), 1500);
      }
    };
    void refresh();
    return () => {
      disposed = true;
      clearTimeout(timer);
      generation.current += 1;
    };
  }, [id]);
  const action = async (input: unknown) => {
    if (mutating.current) return;
    const current = generation.current;
    mutating.current = true;
    sequence.current += 1;
    setBusy(true);
    await run(async () => {
      await request(input);
      if (current !== generation.current) return;
      retry.current = { signature: "", id: "" };
      const read = ++sequence.current;
      const next = await request({ operation: "state" });
      if (current === generation.current && read === sequence.current) {
        setData(next);
        setError("");
      }
    }).finally(() => {
      if (current === generation.current) {
        mutating.current = false;
        setBusy(false);
      }
    });
  };
  const start = (job: FrameworkJob) => {
    const signature = JSON.stringify(job);
    if (retry.current.signature !== signature)
      retry.current = { signature, id: crypto.randomUUID() };
    return action({ operation: "start", requestId: retry.current.id, job });
  };
  return { data, error, busy, action, start };
}
