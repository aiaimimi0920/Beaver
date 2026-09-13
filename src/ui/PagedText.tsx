import { useEffect, useMemo, useRef, useState } from "react";
import { paginateText } from "../shared/pagination";

export function PagedText({
  text,
  label,
  latest = false,
}: {
  text: string;
  label: string;
  latest?: boolean;
}) {
  const ref = useRef<HTMLPreElement>(null);
  const [size, setSize] = useState({ columns: 40, rows: 8 });
  const [page, setPage] = useState(latest ? Number.MAX_SAFE_INTEGER : 0);
  useEffect(() => {
    const observer = new ResizeObserver(() => {
      const el = ref.current;
      if (!el) return;
      const css = getComputedStyle(el);
      setSize({
        columns: Math.max(
          1,
          Math.floor((el.clientWidth - 4) / (parseFloat(css.fontSize) * 1.2)),
        ),
        rows: Math.max(
          1,
          Math.floor((el.clientHeight - 4) / parseFloat(css.lineHeight)),
        ),
      });
    });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  const pages = useMemo(
    () => paginateText(text, size.columns, Math.max(1, size.rows - 1)),
    [text, size],
  );
  const current = Math.min(page, pages.length - 1);
  return (
    <div className="paged-text">
      <pre ref={ref} aria-label={label}>
        {pages[current]}
      </pre>
      <footer className="page-controls">
        <button
          disabled={!current}
          onClick={() => setPage(current - 1)}
          aria-label={`${label}上一页`}
        >
          ‹
        </button>
        <span>
          {current + 1} / {pages.length}
        </span>
        <button
          disabled={current + 1 >= pages.length}
          onClick={() => setPage(current + 1)}
          aria-label={`${label}下一页`}
        >
          ›
        </button>
        <button
          disabled={current + 1 >= pages.length}
          onClick={() => setPage(pages.length - 1)}
        >
          最新
        </button>
      </footer>
    </div>
  );
}
