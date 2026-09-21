import type { FrameworkQuery } from "./use-object-framework-status";

export function FrameworkAvailability({
  query,
  retry,
}: {
  query: FrameworkQuery;
  retry: () => void;
}) {
  if (query.kind === "loading")
    return (
      <section className="op-framework-message" role="status">
        <h2>正在读取项目状态…</h2>
      </section>
    );
  if (query.kind === "error")
    return (
      <section className="op-framework-message" role="alert">
        <h2>无法读取项目状态</h2>
        <p>{query.message}</p>
        <button onClick={retry}>重试</button>
      </section>
    );
  return (
    <section className="op-framework-message" role="status">
      <h2>对象框架尚未接通</h2>
      <p>{query.data.storage.message}</p>
      <ul>
        {query.data.blockers.map((blocker) => (
          <li key={blocker.code}>{blocker.message}</li>
        ))}
      </ul>
      <button onClick={retry}>重新读取</button>
    </section>
  );
}
