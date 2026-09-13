import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  appendNotification,
  notificationLifetime,
  type Notification,
  type NotificationInput,
} from "./notification-state";
import { Icon } from "./Icon";

const NotificationContext = createContext<
  ((input: NotificationInput) => void) | null
>(null);
export function useNotify() {
  const notify = useContext(NotificationContext);
  if (!notify) throw new Error("NotificationProvider is required");
  return notify;
}

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [entries, setEntries] = useState<Notification[]>([]);
  const nextId = useRef(0);
  const notify = useCallback((input: NotificationInput) => {
    const entry = { ...input, id: ++nextId.current, count: 1 };
    setEntries((current) => appendNotification(current, entry));
  }, []);
  const dismiss = useCallback((id: number) => {
    setEntries((current) => current.filter((entry) => entry.id !== id));
  }, []);
  return (
    <NotificationContext.Provider value={notify}>
      {children}
      {createPortal(
        <section className="app-toast-stack" aria-label="通知">
          {entries.map((entry) => (
            <Toast key={entry.id} entry={entry} dismiss={dismiss} />
          ))}
        </section>,
        document.body,
      )}
    </NotificationContext.Provider>
  );
}

function Toast({
  entry,
  dismiss,
}: {
  entry: Notification;
  dismiss: (id: number) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  useEffect(() => {
    const lifetime = notificationLifetime(entry.tone);
    if (lifetime === null || hovered || focused) return;
    const timer = window.setTimeout(() => dismiss(entry.id), lifetime);
    return () => window.clearTimeout(timer);
  }, [entry.id, entry.tone, entry.count, hovered, focused, dismiss]);
  return (
    <div
      className={`app-toast app-toast--${entry.tone}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocusCapture={() => setFocused(true)}
      onBlurCapture={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget))
          setFocused(false);
      }}
    >
      <div
        className="app-toast-message"
        role={entry.tone === "error" ? "alert" : "status"}
        aria-live={entry.tone === "error" ? "assertive" : "polite"}
        aria-atomic="true"
      >
        <span>{entry.text}</span>
        {entry.count > 1 && (
          <small className="app-toast-count">重复 {entry.count} 次</small>
        )}
      </div>
      <button
        className="app-toast-close"
        aria-label="关闭通知"
        title="关闭通知"
        onClick={() => dismiss(entry.id)}
      >
        <Icon name="close" />
      </button>
    </div>
  );
}
