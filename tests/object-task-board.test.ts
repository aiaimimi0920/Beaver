import assert from "node:assert/strict";
import test from "node:test";
import {
  Children,
  isValidElement,
  type ReactNode,
  type ReactElement,
} from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskBoard } from "../src/ui/object-tasks/ObjectTaskBoard";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

type Props = {
  children?: ReactNode;
  "aria-label"?: string;
  disabled?: boolean;
  onClick?: () => void;
};
function elements(node: ReactNode): ReactElement<Props>[] {
  return Children.toArray(node).flatMap((child) =>
    isValidElement<Props>(child)
      ? [child, ...elements(child.props.children)]
      : [],
  );
}
function card(node: ReactNode, label: string) {
  const found = elements(node).find(
    (item) => item.type === "article" && item.props["aria-label"] === label,
  );
  assert.ok(found);
  return found;
}

test("all persisted states and levels appear in lanes without conflating cancellation with failure", () => {
  const snapshot = objectTaskSnapshot();
  const base = snapshot.tasks.find((task) => task.id === "coarse")!;
  snapshot.tasks = (
    [
      "planned",
      "queued",
      "running",
      "awaitingAcceptance",
      "failed",
      "accepted",
      "cancelled",
    ] as const
  ).map((status) => ({ ...base, id: status, title: status, status }));
  const view = ObjectTaskBoard({
    snapshot,
    disabled: false,
    execute: () => {},
  });
  const lanes = elements(view).filter(
    (item) =>
      item.type === "section" &&
      item.props["aria-label"] !== "全层级对象任务泳道",
  );
  assert.equal(lanes.length, 5);
  const counts = lanes.map(
    (lane) => elements(lane).filter((item) => item.type === "article").length,
  );
  assert.deepEqual(counts, [2, 1, 2, 1, 1]);
  assert.match(renderToStaticMarkup(lanes[4]!), /cancelled/);
  assert.doesNotMatch(renderToStaticMarkup(lanes[2]!), /cancelled/);
});

test("parent expansion preserves responsibility while dependencies and independent iterations stay separate", () => {
  const snapshot = objectTaskSnapshot();
  const view = ObjectTaskBoard({
    snapshot,
    disabled: false,
    execute: () => {},
  });
  const coarse = card(view, "粗修 Game");
  const expanded = renderToStaticMarkup(coarse);
  assert.match(expanded, /展开子任务（1）/);
  assert.match(expanded, /Hero/);
  assert.match(expanded, /Movement/);
  assert.doesNotMatch(expanded, /Independent iteration/);
  assert.match(
    renderToStaticMarkup(card(view, "精修 Movement")),
    /依赖：Independent iteration/,
  );
  assert.match(
    renderToStaticMarkup(card(view, "中修 Independent iteration")),
    /独立修改/,
  );
});

test("fine history opens its actual medium session and disabled controls preserve the busy boundary", () => {
  const snapshot = objectTaskSnapshot();
  let session: ObjectTaskExecution | undefined;
  const view = ObjectTaskBoard({
    snapshot,
    disabled: false,
    execute: (task) => {
      session = new ObjectTaskExecution(
        "p",
        snapshot,
        task.id,
        async () => [],
        async () => true,
      );
    },
  });
  const button = elements(card(view, "精修 Movement")).find(
    (item) => item.type === "button",
  )!;
  button.props.onClick?.();
  assert.equal(session?.task.id, "medium");
  assert.deepEqual(
    session?.fineTasks.map((task) => task.id),
    ["fine"],
  );
  const busy = ObjectTaskBoard({
    snapshot,
    disabled: true,
    execute: () => assert.fail("busy callback"),
  });
  assert.ok(
    elements(busy)
      .filter((item) => item.type === "button")
      .every((item) => item.props.disabled),
  );
});

test("accepted stage facts neither complete parents nor count cancellations as accepted or invent unplanned progress", () => {
  const snapshot = objectTaskSnapshot();
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  fine.status = "accepted";
  const render = () =>
    ObjectTaskBoard({ snapshot, disabled: false, execute: () => {} });
  assert.ok(
    renderToStaticMarkup(card(render(), "中修 Hero")).includes(
      "已接受精修 1 / 已规划精修 1",
    ),
  );
  assert.equal(
    snapshot.tasks.find((task) => task.id === "medium")?.status,
    "planned",
  );
  fine.status = "cancelled";
  assert.ok(
    renderToStaticMarkup(card(render(), "中修 Hero")).includes(
      "已接受精修 0 / 已规划精修 1",
    ),
  );
  const html = renderToStaticMarkup(render());
  assert.match(html, /尚未规划精修，进度未知/);
  assert.doesNotMatch(html, /100%|T-100|教室/);
});

test("coarse progress aggregates responsibility descendants and exposes partially unplanned work", () => {
  const snapshot = objectTaskSnapshot();
  const medium = snapshot.tasks.find((task) => task.id === "medium")!;
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  fine.status = "accepted";
  snapshot.tasks.push(
    { ...medium, id: "unplanned", title: "Unplanned object" },
    { ...fine, id: "failed", title: "Failed stage", status: "failed" },
    { ...fine, id: "cancelled", title: "Cancelled stage", status: "cancelled" },
    { ...fine, id: "foreign", parentTaskId: "independent", status: "accepted" },
  );
  const before = JSON.stringify(snapshot);
  const view = ObjectTaskBoard({
    snapshot,
    disabled: false,
    execute: () => {},
  });
  const html = renderToStaticMarkup(card(view, "粗修 Game"));
  assert.match(html, /已接受精修 1 \/ 已规划精修 3/);
  assert.match(html, /失败 1 · 已撤销 1/);
  assert.match(html, /尚未细化子任务（1）：Unplanned object。整体进度未知/);
  assert.match(html, /撤销不算接受，整体完成以父任务验收为准/);
  assert.equal(JSON.stringify(snapshot), before);
});

test("empty coarse and all-cancelled descendants never imply completion", () => {
  const snapshot = objectTaskSnapshot();
  const render = () =>
    renderToStaticMarkup(
      card(
        ObjectTaskBoard({ snapshot, disabled: false, execute: () => {} }),
        "粗修 Game",
      ),
    );
  snapshot.tasks.find((task) => task.id === "fine")!.status = "cancelled";
  assert.match(render(), /已接受精修 0 \/ 已规划精修 1/);
  assert.match(render(), /已撤销 1/);
  snapshot.tasks = snapshot.tasks.filter((task) => task.id === "coarse");
  assert.match(render(), /尚无已规划精修，进度未知/);
  assert.match(render(), /尚未细化子任务（1）：Game/);
  assert.doesNotMatch(render(), /100%/);
});

test("required work counts remain local to the parent scope and optional branches cannot complete it", () => {
  const snapshot = objectTaskSnapshot();
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  const medium = snapshot.tasks.find((task) => task.id === "medium")!;
  fine.status = "cancelled";
  const render = (label: string) =>
    renderToStaticMarkup(
      card(
        ObjectTaskBoard({ snapshot, disabled: false, execute: () => {} }),
        label,
      ),
    );
  assert.match(render("粗修 Game"), /必要精修已接受 0 \/ 1 · 可选范围精修 0/);
  assert.match(render("粗修 Game"), /存在已撤销的必要工作/);
  medium.requirement = "optional";
  assert.match(render("粗修 Game"), /必要精修已接受 0 \/ 0 · 可选范围精修 1/);
  assert.match(render("粗修 Game"), /当前无已规划必要精修，不代表目标已经完成/);
  assert.doesNotMatch(render("粗修 Game"), /存在已撤销的必要工作/);
  assert.match(render("中修 Hero"), /必要精修已接受 0 \/ 1/);
  fine.requirement = "optional";
  assert.match(render("中修 Hero"), /必要精修已接受 0 \/ 0 · 可选范围精修 1/);
  assert.equal(medium.status, "planned");
});
