# Automatic creative decisions

Global settings `askRatio` accepts 0, 10, 30, 70 or 100. Existing installations
default to 100. Tasks accept an optional `askRatio`; null follows the current
global setting. `state` returns each task's computed `effectiveAskRatio`.

The ratios are importance thresholds, not a fixed quota per conversation:
0 automatically chooses every valid recommendation; 10 asks importance above
90; 30 asks above 70; 70 asks above 30; 100 asks all questions.

Each new AI question supplies 2-6 options, the exact recommended option label,
importance from 1 to 100, and its recommendation reason. Invalid recommendation
metadata is rejected for repair rather than inventing an answer. Legacy questions
remain manually answerable. Recommendations and automatic answers are recorded
in the task, prompt and decision history.

## API / MCP

- `settings.save`: include `askRatio` in the normal complete settings payload.
- `task.create`: optional `askRatio`, null means inherit.
- `task.autonomy`: `{ "id": "task-id", "askRatio": 30 }`; use null to inherit.
- `task.answer`: existing `id`, `questionId`, `answers`; optional `automatic`
  array names answers explicitly filled by the recommendation action. Every
  named answer must match a real recommendation eligible under the current
  effective policy. Duplicate IDs and invalid claims are rejected.

Changing settings does not submit an already pending question. In the task UI,
"按当前档位填写推荐答案" fills only empty eligible answers. Existing drafts are
preserved. Users can edit any answer and explicitly submit the batch. Edited
answers are attributed to the user. `answerSources`, `autoAnswers`,
`automaticQuestions`, and `appliedAskRatio` preserve the distinction.

The global selector is in settings; new tasks and existing task conversations
have an inheritance/override selector. The decision-history tab shows automatic
choices, reasons and policy. This governs creative clarification decisions only;
it does not change tool permissions or executable availability.
