export function ObjectTaskPlanningField({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label>
      尚待规划的要求或阶段
      <textarea
        value={value}
        maxLength={4_000}
        onChange={(event) => onChange(event.currentTarget.value)}
        placeholder="记录尚未拆成子任务的需求；可逐行填写。"
      />
      <small>
        保存后在看板提示规划缺口。清空仅表示没有已登记的待规划事项，不代表目标完成。
        上限 4,000 UTF-8 字节；已启动任务仍按冻结定义执行。
      </small>
    </label>
  );
}
