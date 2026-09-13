# Game validation contract

Beaver runs GUT 9.4.0 on the integrated candidate with Godot 4.4 or newer 4.x.
For runtime changes, add meaningful `test_*.gd` behavior and regression tests in
`tests` or `test`. Tests extend `GutTest`; Beaver installs the pinned plugin in
an isolated validation copy. Do not vendor another GUT version. Never delete
assertions, skip failing tests, or narrow required suites to make a failure green.
Documentation-only work does not require artificial tests.

Maintain `beaver.validation.json` (schemaVersion 1) with `code.directories` and
`flows`. Preserve other tasks' flows. Each flow needs `key`, `name`, `category`
(`roaming`, `feature`, `ui`, `task`), `purpose`, `entry` (project-relative scene),
`taskIds` (use the actual current task ID), `config`, `steps`, and `video`.
Use stable unique keys and step IDs. Explain revisions in the task report when
game changes invalidate actions; do not silently skip broken steps.

Example flow shape (replace scene, node and task with actual project values):

```json
{
  "schemaVersion": 1,
  "code": { "directories": ["tests"] },
  "flows": [
    {
      "key": "main-menu",
      "name": "Main menu",
      "category": "ui",
      "purpose": "Show the start menu before entering the game",
      "entry": "main.tscn",
      "taskIds": ["actual-task-id"],
      "config": { "seed": 1, "width": 960, "height": 540, "fps": 30 },
      "steps": [
        {
          "id": "ready",
          "kind": "waitFor",
          "node": "Main/Start",
          "property": "visible",
          "equals": true,
          "timeout": 300
        },
        { "id": "menu", "kind": "capture" }
      ],
      "video": false
    }
  ]
}
```

Actions use real input: `click` with `node`, `key` with numeric Godot `code` and
`pressed` boolean, `action` with input-map `name` and `pressed`; `wait` uses
`frames`. Pair press and release around a wait. `waitFor.timeout` is frames. Prefer bounded
`waitFor` conditions over guessing load time. Capture stable before/after stages.
Use `video: true` for gameplay motion; Beaver records actual rendered frames.
Specify relevant project-relative code, scenes and resources in `references`;
do not invent symbols or exact line numbers. Use `source: "author"` for authored
references. A flow must show its intended task result, not an unrelated screen.

Code validation gates task delivery. Visual capture, comparisons and human
review run independently after delivery. Do not wait for human image approval
to finish a task. Failure feedback includes the tested snapshot and GUT report;
fix the actual cause in this workspace, then Beaver rechecks the new candidate.
