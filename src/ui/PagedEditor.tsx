import { useEffect, useRef, useState } from "react";
import { paginateText } from "../shared/pagination";

export function PagedEditor({
  value,
  change,
  label,
  disabled = false,
}: {
  value: string;
  change: (text: string) => void;
  label: string;
  disabled?: boolean;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const [columns, setColumns] = useState(30);
  const [page, setPage] = useState(0);
  useEffect(() => {
    const observer = new ResizeObserver(() => {
      if (ref.current)
        setColumns(
          Math.max(1, Math.floor((ref.current.clientWidth - 24) / 16)),
        );
    });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  const pages = paginateText(value, columns, 2);
  const current = Math.min(page, pages.length - 1);
  return (
    <div className="paged-editor">
      <textarea
        ref={ref}
        aria-label={label}
        disabled={disabled}
        value={pages[current]}
        onChange={(e) =>
          change(
            pages.slice(0, current).join("") +
              e.target.value +
              pages.slice(current + 1).join(""),
          )
        }
      />
      <div className="page-controls">
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
      </div>
    </div>
  );
}
