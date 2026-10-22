const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

function fixture(status = 'running', text = '  Correct right-side captures.  ') {
  const elements = new Map();
  const element = (id) => {
    if (!elements.has(id)) elements.set(id, { value: '', disabled: false });
    return elements.get(id);
  };
  element('role').value = 'user';
  element('task').value = 'parent-1';
  element('goal').value = text;
  const calls = [];
  const context = {
    document: { getElementById: element, querySelectorAll: () => [] },
    createNprUpgradeActions: () => ({ canUpgrade: () => false }),
    window: { beaver: { call: async (method, input) => {
      calls.push({ method, input });
      if (method === 'state') return { tasks: [{ id: 'parent-1', status }] };
      assert.equal(method, 'task.continue');
      return { status: 'accepted' };
    } } },
  };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, 'renderer.js'), 'utf8'), context);
  return { calls, element };
}

test('steering uses the selected running task, trimmed text and existing context', async () => {
  const f = fixture();
  await f.element('steer').onclick();
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls[0].method, 'state');
  assert.deepEqual(JSON.parse(JSON.stringify(f.calls[1])), {
    method: 'task.continue',
    input: { id: 'parent-1', text: 'Correct right-side captures.', freshContext: false },
  });
});

test('steering refuses non-running task states without sending continuation text', async () => {
  for (const status of ['queued', 'interrupted', 'completed', 'waitingChildren']) {
    const f = fixture(status);
    await f.element('steer').onclick();
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0].method, 'state');
  }
});

test('steering rejects empty text and remains a user-role control', async () => {
  const f = fixture('running', '  ');
  await f.element('steer').onclick();
  assert.equal(f.calls.length, 0);
  f.element('role').value = 'executor';
  f.element('role').onchange();
  assert.equal(f.element('steer').disabled, true);
  f.element('role').value = 'user';
  f.element('role').onchange();
  assert.equal(f.element('steer').disabled, false);
});
