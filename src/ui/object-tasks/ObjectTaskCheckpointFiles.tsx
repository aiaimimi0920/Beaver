import { useState } from "react";
import type { ObjectExecution } from "../../shared/object-attempts";
import type { ObjectAttemptFile } from "./object-attempt-file";
import { ObjectAttemptFilePreview } from "./ObjectAttemptFilePreview";

const pageSize = 100;

export function ObjectTaskCheckpointFiles({
  checkpoints,
  outputCaptured,
  viewer,
}: {
  checkpoints: ObjectExecution["checkpoints"];
  outputCaptured: boolean;
  viewer: ObjectAttemptFile;
}) {
  const [page, setPage] = useState(0);
  const { input, output } = checkpoints;
  const paths = [
    ...new Set([...Object.keys(input), ...Object.keys(output ?? {})]),
  ].sort();
  const pages = Math.max(1, Math.ceil(paths.length / pageSize));
  const currentPage = Math.min(page, pages - 1);
  return (
    <details>
      <summary>查看冻结输入/输出文件清单</summary>
      <p>
        输入 {Object.keys(input).length} 个文件；
        {output === null
          ? "输出清单尚不可用"
          : `输出 ${Object.keys(output).length} 个文件`}
        。 清单来自本次检查点，SHA-256
        表示文件内容摘要；不读取当前工作区，不表示已验收或发布。
      </p>
      {output === null && (
        <p role="status">
          {outputCaptured
            ? "输出已保存，请点击刷新文件清单读取。"
            : "输出尚未冻结，暂不计算新增、修改或删除。"}
        </p>
      )}
      {paths.length === 0 ? (
        <p>
          {output === null ? "输入检查点为空。" : "输入和输出检查点均为空。"}
        </p>
      ) : (
        <table
          aria-label="本次尝试冻结文件对照"
          style={{ overflowWrap: "anywhere" }}
        >
          <thead>
            <tr>
              <th>文件</th>
              <th>变化</th>
              <th>输入 SHA-256</th>
              <th>输出 SHA-256</th>
            </tr>
          </thead>
          <tbody>
            {paths
              .slice(currentPage * pageSize, (currentPage + 1) * pageSize)
              .map((path) => {
                const before = Object.hasOwn(input, path) ? input[path] : null;
                const after =
                  output && Object.hasOwn(output, path) ? output[path] : null;
                const change =
                  output === null
                    ? "待冻结"
                    : before === null
                      ? "新增"
                      : after === null
                        ? "删除"
                        : before === after
                          ? "未变"
                          : "修改";
                return (
                  <tr key={path}>
                    <td>{path}</td>
                    <td>{change}</td>
                    <td>
                      {before ?? "不存在"}
                      {before && (
                        <button
                          onClick={() =>
                            void viewer.open("input", path, before)
                          }
                        >
                          查看输入
                        </button>
                      )}
                    </td>
                    <td>
                      {output === null ? "尚不可用" : (after ?? "不存在")}
                      {after && (
                        <button
                          onClick={() =>
                            void viewer.open("output", path, after)
                          }
                        >
                          查看输出
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
          </tbody>
        </table>
      )}
      {pages > 1 && (
        <nav aria-label="冻结文件分页">
          <button
            disabled={currentPage === 0}
            onClick={() => setPage(currentPage - 1)}
          >
            上一页
          </button>
          <span>
            第 {currentPage + 1} / {pages} 页，共 {paths.length} 项
          </span>
          <button
            disabled={currentPage + 1 === pages}
            onClick={() => setPage(currentPage + 1)}
          >
            下一页
          </button>
        </nav>
      )}
      <ObjectAttemptFilePreview session={viewer} />
    </details>
  );
}
