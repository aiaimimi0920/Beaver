import assert from "node:assert/strict";
import test from "node:test";
import {
  archiveDraft,
  clearDraft,
  readDraft,
  writeDraft,
  type DraftStorage,
  type FeedbackDraft,
} from "../src/ui/asset-task/feedback-draft";

class MemoryStorage implements DraftStorage {
  readonly values = new Map<string, string>();
  failWrite = false;
  failRemove = false;
  getItem(key: string) {
    return this.values.get(key) ?? null;
  }
  setItem(key: string, value: string) {
    if (this.failWrite) throw new Error("storage full");
    this.values.set(key, value);
  }
  removeItem(key: string) {
    if (this.failRemove) throw new Error("storage unavailable");
    this.values.delete(key);
  }
}

const key = "beaver.asset-feedback.v1.task-1";
function draft(): FeedbackDraft {
  return {
    version: 1,
    submission: {
      id: "task-1",
      feedbackId: "feedback-1",
      referenceId: "reference-1",
      text: "Make the nose smaller",
      timing: "now",
      annotations: [
        {
          kind: "box",
          points: [
            [0.2, 0.3],
            [0.4, 0.5],
          ],
        },
      ],
    },
    reference: {
      id: "reference-1",
      taskId: "task-1",
      projectId: "project-1",
      sha256: "0".repeat(64),
      used: false,
      frame: {
        id: "frame-1",
        sessionId: "session-1",
        generation: "scene-1",
        sceneRevision: 2,
        viewRevision: 3,
        capturedAt: 1000,
        width: 720,
        height: 540,
        viewMatrix: Array<number>(16).fill(0),
        projectionMatrix: Array<number>(16).fill(0),
      },
      pick: {
        frameId: "frame-1",
        objectId: "mesh-1",
        objectName: "Body",
        instanceId: "instance-1",
        local: [0, 0, 1],
        world: [0, 1, 1],
        normal: [0, -1, 0],
        face: 5,
        vertices: 30,
        polygons: 20,
        point: [0.3, 0.4],
      },
    },
  };
}

test("persisted retry retains the same ID, image, pick and annotations after UI edits", () => {
  const storage = new MemoryStorage();
  const input = draft();
  const expected = structuredClone(input);
  const fixed = writeDraft(storage, input);
  input.submission.feedbackId = "second-request";
  input.submission.text = "Different edit";
  const point = input.submission.annotations[0]?.points[0];
  assert.ok(point);
  point[0] = 0.9;
  input.reference.frame.sceneRevision = 50;
  input.reference.pick!.local[2] = 9;
  assert.deepEqual(fixed, expected);
  assert.deepEqual(readDraft(storage, "task-1"), expected);
  assert.equal(readDraft(storage, "unrelated-task"), null);
  clearDraft(storage, "task-1");
  assert.equal(readDraft(storage, "task-1"), null);
});

test("cross-task and cross-frame drafts cannot replace a valid pending request", () => {
  const storage = new MemoryStorage();
  const valid = writeDraft(storage, draft());
  const changes: ((value: FeedbackDraft) => void)[] = [
    (value) => {
      value.reference.taskId = "another-task";
    },
    (value) => {
      value.reference.id = "another-reference";
    },
    (value) => {
      value.reference.pick!.frameId = "another-frame";
    },
    (value) => {
      const point = value.submission.annotations[0]?.points[0];
      assert.ok(point);
      point[0] = -0.1;
    },
    (value) => {
      value.reference.pick!.local[0] = Infinity;
    },
    (value) => {
      value.reference.frame.width = 100000;
    },
  ];
  for (const change of changes) {
    const invalid = draft();
    change(invalid);
    assert.throws(() => writeDraft(storage, invalid));
    assert.deepEqual(readDraft(storage, "task-1"), valid);
  }
  const wrongTask = draft();
  wrongTask.submission.id = "another-task";
  storage.setItem(key, JSON.stringify(wrongTask));
  assert.throws(() => readDraft(storage, "task-1"));
});

test("corrupt and oversized drafts remain available for explicit recovery", () => {
  const storage = new MemoryStorage();
  for (const corrupt of ["{broken", "x".repeat(300001)]) {
    storage.setItem(key, corrupt);
    assert.throws(() => readDraft(storage, "task-1"));
    assert.equal(storage.getItem(key), corrupt);
    archiveDraft(storage, "task-1");
    assert.equal(storage.getItem(`${key}.recovery`), corrupt);
    assert.equal(storage.getItem(key), null);
  }
});

test("storage failures never silently discard the only pending request", () => {
  const storage = new MemoryStorage();
  writeDraft(storage, draft());
  const original = storage.getItem(key);
  storage.failWrite = true;
  const replacement = draft();
  replacement.submission.feedbackId = "replacement";
  assert.throws(() => writeDraft(storage, replacement), /storage full/);
  assert.throws(() => archiveDraft(storage, "task-1"), /storage full/);
  assert.equal(storage.getItem(key), original);
  storage.failWrite = false;
  storage.failRemove = true;
  assert.throws(() => archiveDraft(storage, "task-1"), /storage unavailable/);
  assert.equal(storage.getItem(key), original);
  assert.equal(storage.getItem(`${key}.recovery`), original);
  storage.failRemove = false;
  archiveDraft(storage, "task-1");
  assert.equal(storage.getItem(key), null);
});
