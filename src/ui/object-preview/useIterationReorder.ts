import { useEffect, useRef, useState, type ButtonHTMLAttributes } from "react";
import { moveMockIteration, objectIterations } from "./mock-iterations";

export type IterationDragHandle = Pick<
  ButtonHTMLAttributes<HTMLButtonElement>,
  | "onPointerDown"
  | "onPointerMove"
  | "onPointerUp"
  | "onPointerCancel"
  | "onLostPointerCapture"
  | "onKeyDown"
>;

interface DragPreview {
  taskId: string;
  beforeId: string | null;
  valid: boolean;
}

interface DragGesture extends DragPreview {
  pointerId: number;
  handle: HTMLButtonElement;
  startX: number;
  startY: number;
  x: number;
  y: number;
  active: boolean;
}

export function useIterationReorder(objectId: string, onStart: () => void) {
  const listRef = useRef<HTMLOListElement>(null);
  const gesture = useRef<DragGesture | null>(null);
  const frame = useRef(0);
  const [preview, setPreview] = useState<DragPreview | null>(null);
  const [, refresh] = useState(0);

  const cancel = () => {
    const current = gesture.current;
    gesture.current = null;
    cancelAnimationFrame(frame.current);
    if (current?.handle.hasPointerCapture(current.pointerId))
      current.handle.releasePointerCapture(current.pointerId);
    setPreview(null);
  };

  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || !gesture.current) return;
      event.preventDefault();
      event.stopPropagation();
      cancel();
    };
    window.addEventListener("keydown", escape, true);
    window.addEventListener("blur", cancel);
    return () => {
      window.removeEventListener("keydown", escape, true);
      window.removeEventListener("blur", cancel);
      cancel();
    };
  }, [objectId]);

  const locate = () => {
    const current = gesture.current;
    const list = listRef.current;
    if (!current?.active || !list) return;
    const scroll = list.closest<HTMLElement>(".op-inspector-scroll");
    const bounds = (scroll ?? list).getBoundingClientRect();
    current.valid =
      current.x >= bounds.left &&
      current.x <= bounds.right &&
      current.y >= bounds.top &&
      current.y <= bounds.bottom;
    if (current.valid) {
      const ghost = list
        .querySelector(".op-iteration-drop-preview")
        ?.getBoundingClientRect();
      if (!ghost || current.y < ghost.top || current.y > ghost.bottom) {
        const rows = Array.from(
          list.querySelectorAll<HTMLElement>("[data-iteration-id]"),
        );
        current.beforeId =
          rows.find((row) => {
            if (row.dataset.iterationId === current.taskId) return false;
            const rect = row.getBoundingClientRect();
            return current.y < rect.top + rect.height / 2;
          })?.dataset.iterationId ?? null;
      }
    }
    setPreview((value) =>
      value?.taskId === current.taskId &&
      value.beforeId === current.beforeId &&
      value.valid === current.valid
        ? value
        : {
            taskId: current.taskId,
            beforeId: current.beforeId,
            valid: current.valid,
          },
    );
  };

  const autoScroll = () => {
    const current = gesture.current;
    if (!current?.active) return;
    const scroll = listRef.current?.closest<HTMLElement>(
      ".op-inspector-scroll",
    );
    if (scroll) {
      const rect = scroll.getBoundingClientRect();
      if (
        current.x >= rect.left &&
        current.x <= rect.right &&
        current.y >= rect.top &&
        current.y <= rect.bottom
      ) {
        const edge = 44;
        const delta =
          current.y < rect.top + edge
            ? -Math.ceil((rect.top + edge - current.y) / 4)
            : current.y > rect.bottom - edge
              ? Math.ceil((current.y - rect.bottom + edge) / 4)
              : 0;
        if (delta) {
          scroll.scrollTop += delta;
          locate();
        }
      }
    }
    frame.current = requestAnimationFrame(autoScroll);
  };

  const handle = (taskId: string): IterationDragHandle => ({
    onPointerDown: (event) => {
      if (event.button !== 0 || !event.isPrimary || gesture.current) return;
      event.preventDefault();
      event.currentTarget.focus();
      event.currentTarget.setPointerCapture(event.pointerId);
      gesture.current = {
        taskId,
        pointerId: event.pointerId,
        handle: event.currentTarget,
        startX: event.clientX,
        startY: event.clientY,
        x: event.clientX,
        y: event.clientY,
        beforeId: null,
        valid: false,
        active: false,
      };
    },
    onPointerMove: (event) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) return;
      current.x = event.clientX;
      current.y = event.clientY;
      if (!current.active) {
        if (
          Math.hypot(current.x - current.startX, current.y - current.startY) < 5
        )
          return;
        current.active = true;
        onStart();
        frame.current = requestAnimationFrame(autoScroll);
      }
      locate();
    },
    onPointerUp: (event) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) return;
      current.x = event.clientX;
      current.y = event.clientY;
      locate();
      if (current.active && current.valid)
        moveMockIteration(objectId, current.taskId, current.beforeId);
      cancel();
    },
    onPointerCancel: cancel,
    onLostPointerCapture: cancel,
    onKeyDown: (event) => {
      if (gesture.current || !["ArrowUp", "ArrowDown"].includes(event.key))
        return;
      event.preventDefault();
      const tasks = objectIterations(objectId);
      const index = tasks.findIndex((task) => task.id === taskId);
      const next = index + (event.key === "ArrowUp" ? -1 : 1);
      if (index < 0 || next < 0 || next >= tasks.length) return;
      onStart();
      const beforeId =
        event.key === "ArrowUp"
          ? tasks[next]!.id
          : (tasks[next + 1]?.id ?? null);
      moveMockIteration(objectId, taskId, beforeId);
      refresh((value) => value + 1);
    },
  });

  return { listRef, preview, handle };
}
