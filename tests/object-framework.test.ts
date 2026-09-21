import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { z } from "zod";
import {
  objectTaskIdentitySchema,
  parseObjectFrameworkStatus,
} from "../src/shared/object-framework";
import { FrameworkAvailability } from "../src/ui/object-preview/FrameworkAvailability";
import { ObjectFrameworkWorkspace } from "../src/ui/object-preview/ObjectFrameworkWorkspace";

const samples = z
  .object({ accepted: z.array(z.unknown()), rejected: z.array(z.unknown()) })
  .parse(
    JSON.parse(
      fs.readFileSync(
        new URL("./fixtures/object-framework-identities.json", import.meta.url),
        "utf8",
      ),
    ),
  );
const readiness = {
  schemaVersion: 1,
  projectId: "project",
  storage: {
    state: "legacy",
    message: "项目尚未启用 .beaver 存储",
    manifestPath: ".beaver/project.json",
    databasePath: ".beaver/project.sqlite",
  },
  capabilities: {
    objectsRead: false,
    manufactureRead: false,
    execution: false,
  },
  blockers: [
    {
      code: "OBJECT_FRAMEWORK_DISABLED",
      message: "对象队列和阶段门槛尚未接通",
    },
  ],
};

test("UI and native share versioned identity samples without accepting ambiguous baselines", () => {
  for (const sample of samples.accepted)
    assert.deepEqual(objectTaskIdentitySchema.parse(sample), sample);
  for (const sample of samples.rejected)
    assert.equal(objectTaskIdentitySchema.safeParse(sample).success, false);
  for (const length of [128, 129]) {
    assert.equal(
      objectTaskIdentitySchema.safeParse({
        schemaVersion: 1,
        layer: "medium",
        objectId: "a".repeat(length),
        baseline: { basePolicy: "empty" },
      }).success,
      length === 128,
    );
  }
});

test("readiness rejects a different project, unknown storage version and incomplete response", () => {
  assert.equal(
    parseObjectFrameworkStatus(readiness, "project").storage.state,
    "legacy",
  );
  assert.throws(
    () => parseObjectFrameworkStatus(readiness, "other"),
    /项目状态响应/,
  );
  assert.throws(() =>
    parseObjectFrameworkStatus({ ...readiness, schemaVersion: 2 }, "project"),
  );
  assert.throws(() =>
    parseObjectFrameworkStatus({ ...readiness, capabilities: {} }, "project"),
  );
});

test("production pages have honest project/loading states and disabled unsupplied actions", () => {
  for (const page of ["objects", "manufacture"] as const) {
    const empty = renderToStaticMarkup(
      createElement(ObjectFrameworkWorkspace, {
        loading: false,
        page,
        createProject: () => {},
      }),
    );
    assert.match(empty, /尚未选择项目/);
    assert.match(empty, /创建项目/);
    assert.doesNotMatch(empty, /放学后|demo-|浅野|正在制作|%/);
    if (page === "objects") {
      for (const label of ["导入对象", "生成对象", "点选画面", "框选画面"]) {
        assert.match(
          empty,
          new RegExp(`<button[^>]*aria-label="${label}"[^>]*disabled=""`),
        );
      }
    }
    const loading = renderToStaticMarkup(
      createElement(ObjectFrameworkWorkspace, {
        loading: true,
        page,
        createProject: () => {},
      }),
    );
    assert.match(loading, /正在打开本地工作室/);
    assert.doesNotMatch(loading, /创建项目/);
  }
});

test("read failures and disabled storage remain distinguishable and offer retry", () => {
  const failure = renderToStaticMarkup(
    createElement(FrameworkAvailability, {
      query: { kind: "error", message: "Connection unavailable <script>" },
      retry: () => {},
    }),
  );
  assert.match(failure, /role="alert"/);
  assert.match(failure, /Connection unavailable &lt;script&gt;/);
  assert.match(failure, /重试/);
  const unavailable = renderToStaticMarkup(
    createElement(FrameworkAvailability, {
      query: {
        kind: "loaded",
        data: parseObjectFrameworkStatus(readiness, "project"),
      },
      retry: () => {},
    }),
  );
  assert.match(unavailable, /项目尚未启用/);
  assert.match(unavailable, /对象队列和阶段门槛尚未接通/);
  assert.doesNotMatch(unavailable, /暂无对象|成功/);
});
